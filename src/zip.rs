use std::io::{Read, Write};
use flate2::write::GzEncoder;

pub fn from_encoder<R: Read, W: Write>(mut input: R, mut gz: GzEncoder<W>) -> std::io::Result<W> {
    let mut inbuf: Vec<u8> = Vec::new();
    input.read_to_end(&mut inbuf)?;
    gz.write_all(inbuf.as_slice())?;
    return gz.finish();
}
