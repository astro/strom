extern crate hyper;
extern crate gst;

use hyper::server::{Server, Request, Response, Handler};
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel};
use hyper::method::Method;
use hyper::status::StatusCode;
use hyper::uri::RequestUri;
use hyper::net::Fresh;
use std::io::Write;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod chunk_reader;
mod pipe;
mod source;
mod sink;
mod broadcast;
mod streams;

use streams::Stream;


struct HttpHandler {
    streams: Mutex<HashMap<String, Arc<Mutex<streams::Stream>>>>
}

impl HttpHandler {
    pub fn new() -> Self {
        HttpHandler {
            streams: Mutex::new(HashMap::new())
        }
    }
}

// TODO: move Read & Write to Sink/Source
impl Handler for HttpHandler {
    fn handle<'a, 'k>(&'a self, req: Request<'a, 'k>, mut res: Response<'a, Fresh>) {
        println!("req: {:?} {:?}", req.method, req.uri);
        let path = match &req.uri {
            &RequestUri::AbsolutePath(ref path) => path.clone(),
            _ => panic!("Path!")
        };
        let path_components: Vec<String> = path.split(|c| c == '/')
            .skip(1)
            .map(|s| s.to_owned())
            .collect();

        if req.method == Method::Put || req.method == Method::Post {
            res.headers_mut().set(ContentType(
                Mime(TopLevel::Text, SubLevel::Html, vec![])
            ));
            let res = res.start().unwrap();

            let stream = Arc::new(Mutex::new(streams::Stream::new()));
            {
                let mut streams = self.streams.lock().unwrap();
                streams.insert(path_components[0].clone(), stream.clone());
                println!("New stream: {}", path_components[0]);
            }

            let mut source = {
                let mut stream = stream.lock().unwrap();
                stream.get_source()
            };
            source.read_from(req);
            println!("Done!");

            {
                let mut streams = self.streams.lock().unwrap();
                streams.remove(&path_components[0]);
                println!("Removed stream: {}", path_components[0]);
            }
            res.end().unwrap();
        } else if req.method == Method::Get {
            res.headers_mut().set(ContentType(
                Mime(TopLevel::Video, SubLevel::Ext("mp4".to_owned()), vec![])
            ));
            let mut res = res.start().unwrap();

            let consume = {
                let streams = self.streams.lock().unwrap();
                streams.get(&path_components[0])
                    .and_then(|stream| {
                        println!("Found a valid stream to this req!");
                        let mut stream = stream.lock().unwrap();
                        match stream.get_mux_consumer(&path_components[1]) {
                            Ok(stream) => Some(stream),
                            Err(e) => {
                                println!("Cannot get mux consumer: {}", e);
                                None
                            }
                        }
                    })
            };
            match consume {
                Some(mut consume) => {
                    println!("Got consumer for {:?}", path_components);
                    consume.write_to(&mut res);
                },
                None =>
                    println!("Got no consumer for {:?}", path_components)
            }
            println!("Ending");
            res.end().unwrap_or(());
            println!("Ended");
        } else {
            *res.status_mut() = StatusCode::BadRequest;
            res.start().unwrap().end().unwrap_or(());
        }
    }
}

fn main() {
    gst::init();
    let mut mainloop = gst::MainLoop::new();
    mainloop.spawn();

    let handler = HttpHandler::new();
    Server::http("0.0.0.0:8080").unwrap().handle(handler).unwrap();

    mainloop.quit();
}
