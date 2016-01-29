use pipe::Pipe;
use source::Source;
use gst;
use gst::{ElementT, BinT};

use sink::Sink;

#[allow(dead_code)]
pub struct Stream {
    // Never retrieved, but referenced to stay alive:
    pipe: Pipe,
    src: Source
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
            src: pipe.appsrc("input").expect("by appsrc"),
            pipe: pipe
        }
    }

    pub fn push_buffer(&mut self, buffer: &[u8]) {
        self.src.push_buffer(buffer);
    }

    pub fn push_end(&mut self) {
        self.src.end();
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
        // TODO: fnord* must be unique!
        let mut queue = try!(gst::Element::factory_make("queue", "fnord0")
            .ok_or("Cannot create queue"));
        try!(self.pipe.add(queue.to_element()));
        let mut muxer = try!(gst::Element::factory_make(muxer_element, "fnord1")
            .ok_or("Cannot create muxer"));
        for (k, v) in muxer_config {
            muxer.set(k, v);
        }
        try!(self.pipe.add(muxer.to_element()));
        let mut appsink = try!(gst::Element::factory_make("appsink", "fnord2")
            .ok_or("Cannot create appsink"));
        try!(self.pipe.add(appsink.to_element()));

        // Link
        let mut tee = try!(self.pipe.get(tee_name)
            .ok_or("Cannot get tee"));
        if !tee.link(&mut queue) {
            panic!("Cannot link tee to queue");
            return Err(format!("Cannot link tee to queue"));
        }
        queue.set_state(gst::GST_STATE_PLAYING);
        if !queue.link(&mut muxer) {
            panic!("Cannot link tee to {}", muxer_element);
            return Err(format!("Cannot link tee to {}", muxer_element));
        }
        muxer.set_state(gst::GST_STATE_PLAYING);
        if !muxer.link(&mut appsink) {
            panic!("Cannot link {} to appsink", muxer_element);
            return Err(format!("Cannot link {} to appsink", muxer_element));
        }
        appsink.set_state(gst::GST_STATE_PLAYING);

        Ok(Consumer {
            muxer: muxer,
            sink: Sink::new(appsink)
        })
    }
}

pub struct Consumer {
    muxer: gst::Element,
    sink: Sink
}

/// Facade over Sink
impl Iterator for Consumer {
    type Item = gst::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.sink.next()
    }
}
