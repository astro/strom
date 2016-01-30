use gst;
use std::sync::Arc;
use std::io::Read;

use chunk_reader::ChunkReader;

pub struct Source {
    src: gst::appsrc::AppSrc,
    pool: Option<(usize, Arc<gst::BufferPool>)>
}

impl Source {
    pub fn new(el: gst::Element) -> Self {
        let appsrc = gst::appsrc::AppSrc::new_from_element(el);

        Source {
            src: appsrc,
            pool: None
        }
    }

    fn need_pool(&mut self, size: usize) -> Arc<gst::BufferPool> {
        match self.pool {
            Some((actual_size, ref pool)) if size == actual_size => return pool.clone(),
            _ => {
                let pool = gst::BufferPool::new().unwrap();
                // let src_caps = self.src.caps().expect("AppSrc Element caps");
                let src_caps = gst::Caps::from_string("video/x-raw").unwrap();
                pool.set_params(&src_caps, size as u32, 0, 0);
                pool.set_active(true).expect("Activate bufferpool");
                let arc = Arc::new(pool);
                println!("New bufferpool with {}", size);
                self.pool = Some((size, arc.clone()));
                arc
            }
        }
    }

    pub fn write(&mut self, data: &[u8]) -> bool {
        let pool = self.need_pool(data.len());

        let mut buffer = pool.acquire_buffer().expect("acquire buffer");
        match buffer.map_write(|mut mapping| {
            for (i, c) in mapping.iter_mut::<u8>().enumerate() {
                *c = data[i];
            }
        }) {
            Ok(()) => (),
            Err(e) => return false
        }
        buffer.set_live(true);
        let res = self.src.push_buffer(buffer);
        res == 0
    }

    pub fn end(&mut self) {
        let res = self.src.end_of_stream();
        println!("EOF return: {}", res);
    }

    pub fn read_from<R: Read>(&mut self, r: R) {
        for chunk in ChunkReader::new(r, 8192) {
            if !self.write(&chunk) {
                break;
            }
        }
        self.end();
    }
}
