use pipe::Pipe;
use source::Source;
use broadcast::*;

#[allow(dead_code)]
pub struct Stream {
    // Never retrieved, but referenced to stay alive:
    pipe: Pipe,
    src: Source,
    broadcasts: Vec<(&'static str, Broadcast)>
}

impl Stream {
    /**
    * s: "matroskademux ! matroskamux streamable=true"
    **/
    pub fn new() -> Self {
        let pipe = Pipe::new(
            r"
appsrc name=input input. ! decodebin ! audio/x-raw ! tee name=t
t. ! queue ! audioconvert ! lamemp3enc bitrate=6 ! appsink name=mp3
t. ! queue ! audioconvert ! vorbisenc bitrate=128000 ! oggmux ! appsink name=ogg
t. ! queue ! audioconvert ! voaacenc bitrate=80000 ! mp4mux fragment-duration=1000 ! appsink name=m4a
t. ! queue ! audioconvert ! opusenc bitrate=64000 ! oggmux ! appsink name=opus
"
        ).expect("New pipeline failed");

        let broadcasts = ["mp3", "ogg", "m4a", "opus"]
            .into_iter()
            .map(|name| (name.to_owned(), Broadcast::new(pipe.appsink(&name).expect("by appsink"))) )
            .collect();

        Stream {
            src: pipe.appsrc("input").expect("by appsrc"),
            pipe: pipe,
            broadcasts: broadcasts
        }
    }

    pub fn push_buffer(&mut self, buffer: &[u8]) {
        self.src.push_buffer(buffer);
    }

    pub fn push_end(&mut self) {
        self.src.end();
    }

    pub fn get_broadcast_consumer(&mut self, wanted_name: &str) -> Option<Consumer> {
        self.broadcasts
            .iter_mut()
            .find(|&&mut (ref name, _)| name == &wanted_name)
            .and_then(|&mut (_, ref mut broadcast)| Some(broadcast.consumer()))
    }
}
