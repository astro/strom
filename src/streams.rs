use gst;
use gst::{ElementT, BinT};
use std::io::Write;

use sink::Sink;
use pipe::Pipe;
use source::Source;

#[allow(dead_code)]
pub struct Stream {
    // Never retrieved, but referenced to stay alive:
    pipe: Pipe
}

impl Stream {
    /**
    * s: "matroskademux ! matroskamux streamable=true"
    **/
    pub fn new() -> Self {
        let pipe = Pipe::new(
            r"
appsrc name=input input. ! decodebin ! audio/x-raw ! tee name=t allow-not-linked=true
t. ! queue ! audioconvert ! lamemp3enc quality=6 ! tee name=mp3 allow-not-linked=true
t. ! queue ! audioconvert ! vorbisenc bitrate=128000 ! tee name=ogg allow-not-linked=true
t. ! queue ! audioconvert ! voaacenc bitrate=80000 ! tee name=m4a allow-not-linked=true
t. ! queue ! audioconvert ! opusenc bitrate=64000 ! tee name=opus allow-not-linked=true
"
        ).expect("New pipeline failed");

        Stream {
            pipe: pipe
        }
    }

    pub fn get_source(&self) -> Source {
        self.pipe.appsrc("input").expect("input appsrc")
    }

    pub fn get_mux_consumer(&mut self, tee_name: &str) -> Result<Consumer, String> {
        // Setup
        let (muxer_element, muxer_config) = try!(match tee_name {
            "mp3" => Ok(("mpegpsmux", vec![])),
            "ogg" => Ok(("oggmux", vec![])),
            "m4a" => Ok(("mp4mux", vec![("fragment-duration", 1000)])),
            "opus" => Ok(("oggmux", vec![])),
            _ => Err("No such stream")
        });

        let mut bin = try!(gst::Bin::new("")
            .ok_or("Cannot create bin"));

        let mut queue = try!(gst::Element::new("queue", "")
            .ok_or("Cannot create queue"));
        if !bin.add(queue.to_element()) {
            return Err("Cannot add queue to bin".to_owned());
        }
        let mut muxer = try!(gst::Element::new(muxer_element, "")
            .ok_or("Cannot create muxer"));
        for (k, v) in muxer_config {
            muxer.set(k, v);
        }
        if !bin.add(muxer.to_element()) {
            return Err("Cannot add muxer to bin".to_owned());
        }
        let mut appsink = try!(gst::Element::new("appsink", "")
            .ok_or("Cannot create appsink"));
        if !bin.add(appsink.to_element()) {
            return Err("Cannot add appsink to bin".to_owned());
        }

        self.pipe.add(bin.to_element());

        // Link
        let mut tee = try!(self.pipe.get(tee_name)
            .ok_or("Cannot get tee"));
        if !tee.link(&mut queue) {
            return Err("Cannot link tee to queue".to_owned());
        }
        if !queue.link(&mut muxer) {
            return Err(format!("Cannot link tee to {}", muxer_element));
        }
        if !muxer.link(&mut appsink) {
            return Err(format!("Cannot link {} to appsink", muxer_element));
        }

        bin.set_state(gst::GST_STATE_PLAYING);

        Ok(Consumer {
            sink: Sink::new(appsink),
            bin: bin,
            tee: tee,
            queue: queue,
            parent: self.pipe.clone()
        })
    }
}

pub struct Consumer {
    sink: Sink,
    bin: gst::Bin,
    tee: gst::Element,
    queue: gst::Element,
    parent: Pipe
}

impl Drop for Consumer {
    fn drop(&mut self) {
        self.tee.unlink(&mut self.queue);
        self.bin.pause();
        self.parent.remove(&self.bin)
            .unwrap_or_else(|e| println!("{}", e) )
    }
}

/// Facade over Sink
impl Iterator for Consumer {
    type Item = gst::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.sink.next()
    }
}

impl Consumer {
    pub fn write_to<W: Write>(&mut self, w: &mut W) {
        for sample in self {
            let buffer = sample.buffer().unwrap();
            let mut data = Vec::with_capacity(buffer.size() as usize);
            unsafe { data.set_len(buffer.size() as usize); }
            match buffer.map_read(|mapping| {
                for (i, c) in mapping.iter::<u8>().enumerate() {
                    data[i] = *c;
                }
            }) {
                Ok(()) => (),
                Err(e) => {
                    println!("{:?}", e);
                    break;
                }
            }
            match w.write(&data) {
                Ok(size) if size == data.len() =>
                    (),
                Ok(size) => {
                    println!("Wrote only {} of {} bytes", size, data.len());
                    break;
                },
                Err(e) => {
                    println!("Error writing {} bytes: {}", data.len(), e);
                    break;
                }
            }
        }
    }
}
