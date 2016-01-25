use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use gst;

use sink::Sink;

type Consumers = Arc<Mutex<Vec<Sender<Arc<Mutex<gst::Sample>>>>>>;

pub struct Broadcast {
    consumers: Consumers
}

impl Broadcast {
    pub fn new(sink: Sink) -> Self {
        let consumers = Arc::new(Mutex::new(vec![]));
        let self_ = Broadcast {
            consumers: consumers.clone()
        };
        thread::spawn(|| Self::run_distributer(consumers, sink) );
        self_
    }

    fn run_distributer(consumers: Consumers, sink: Sink) {
        for sample in sink {
            let sample = Arc::new(Mutex::new(sample));
            let consumers = consumers.lock().expect("consumers lock");
            for tx in consumers.iter() {
                tx.send(sample.clone()).unwrap();
            }
        }
    }

    pub fn consumer(&self) -> Consumer {
        let (tx, rx) = channel();
        
        let mut consumers = self.consumers.lock().unwrap();
        consumers.push(tx);
        
        Consumer { rx: rx, sync: false }
    }
}

pub struct Consumer {
    rx: Receiver<Arc<Mutex<gst::Sample>>>,
    sync: bool
}

impl Iterator for Consumer {
    type Item = Arc<Mutex<gst::Sample>>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.rx.recv() {
            Err(e) => {
                println!("Consumer error: {:?}", e);
                None
            },
            Ok(sample) => {
                if self.sync {
                    Some(sample)
                } else {
                    let is_sync = {
                        let buffer = sample
                            .lock()
                            .unwrap()
                            .buffer()
                            .unwrap();
                        buffer_is_sync_frame(&buffer)
                    };
                    if is_sync {
                        self.sync = true;
                        Some(sample)
                    } else {
                        // Retry next:
                        self.next()
                    }
                }
            }
        }
    }
}

impl Drop for Consumer {
    fn drop(&mut self) {
        println!("TODO");
    }
}

// See: http://cgit.freedesktop.org/gstreamer/gst-plugins-base/tree/gst/tcp/gstmultihandlesink.c#n1141
fn buffer_is_sync_frame(buffer: &gst::Buffer) -> bool {
    if buffer.is_delta_unit() {
        false
    } else if !buffer.is_header() {
        true
    } else {
        false
    }
}
