use gst;
use gst::{BinT, ElementT};
use std::thread;

use source::Source;
use sink::Sink;


pub struct Pipe {
    pipeline: gst::Pipeline
}

impl Pipe {
    /**
    * s: "matroskademux ! matroskamux streamable=true"
    **/
    pub fn new(pipe_str: &str) -> Option<Self> {
        let mut pipeline = match gst::Pipeline::new_from_str(pipe_str) {
            Ok(pipeline) => pipeline,
            Err(e) => {
                println!("New pipeline error: {:?}", e);
                return None
            }
        };

        let mut bus = pipeline.bus().expect("Couldn't get bus from pipeline");
        thread::spawn(move || {
	    let bus_receiver = bus.receiver();
            for message in bus_receiver.iter() {
                match message.parse() {
                    gst::Message::StateChangedParsed{msg: _, ref old, ref new, pending: _} =>
			            println!("element `{}` changed from {:?} to {:?}", message.src_name(), old, new),
                    gst::Message::StateChanged(_) =>
			            println!("element `{}` state changed", message.src_name()),
		            gst::Message::ErrorParsed{msg: _, ref error, debug: _} => {
                        println!("error msg from element `{}`: {}, quitting", message.src_name(), error.message());
	                    break;
                    },
		            gst::Message::Eos(ref _msg) => {
                        println!("eos received quiting");
	                    break;
                    },
                    _ =>
                        println!("Pipe message: {} from {} at {}", message.type_name(), message.src_name(), message.timestamp())
                }
            }
        });

        pipeline.play();

        Some(Pipe {
            pipeline: pipeline
        })
    }

    pub fn appsrc(&self, name: &str) -> Option<Source> {
        self.pipeline.get_by_name(name).and_then(|el| Some(Source::new(el)) )
    }

    pub fn appsink(&self, name: &str) -> Option<Sink> {
        self.pipeline.get_by_name(name).and_then(|el| Some(Sink::new(el)) )
    }

    pub fn add<E: gst::ElementT>(&mut self, element: E) -> Result<(), &'static str> {
        match self.pipeline.add(element) {
            true => Ok(()),
            false => Err("Cannot add element to pipeline")
        }
    }

    pub fn get(&self, name: &str) -> Option<gst::Element> {
        self.pipeline.get_by_name(name)
    }
}
