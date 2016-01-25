use gst;

pub struct Sink {
    sink: gst::appsink::AppSink
}

impl Sink {
    pub fn new(el: gst::Element) -> Self {
        let appsink = gst::appsink::AppSink::new_from_element(el);

        Sink {
            sink: appsink
        }
    }
}

impl Iterator for Sink {
    type Item = gst::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        match self.sink.recv() {
            Ok(gst::appsink::Message::NewPreroll(sample)) =>
                Some(sample),
            Ok(gst::appsink::Message::NewSample(sample)) =>
                Some(sample),
            Ok(gst::appsink::Message::Eos) =>
                None,
            Err(e) => {
                println!("Sink error: {}", e);
                None
            }
        }
    }
}
