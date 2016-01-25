use std::io::Read;

pub struct ChunkReader<R: Read> {
    r: R,
    chunk_size: usize
}

impl<R: Read> ChunkReader<R> {
    pub fn new(r: R, chunk_size: usize) -> Self {
        ChunkReader {
            r: r,
            chunk_size: chunk_size
        }
    }
    
    pub fn read_chunk(&mut self) -> Option<Vec<u8>> {
        let mut offset = 0;
        let mut buf = Vec::with_capacity(self.chunk_size);
        unsafe { buf.set_len(self.chunk_size); }

        while offset < buf.len() {
            match self.r.read(&mut buf[offset..]) {
                Ok(0) => break,
                Ok(size) => offset += size,
                Err(e) => {
                    println!("{}", e);
                    break
                }
            }
        }

        if offset > 0 {
            if offset < buf.len() {
                buf.truncate(offset);
            }
            Some(buf)
        } else {
            None
        }
    }
}

impl<R: Read> Iterator for ChunkReader<R> {
    type Item = Vec<u8>;
        
    fn next(&mut self) -> Option<Self::Item> {
        self.read_chunk()
    }
}
