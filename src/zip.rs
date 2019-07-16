use std::io::{Read, Write};
use flate2::{Compression, GzBuilder};
use flate2::write::GzEncoder;

pub fn from<R: Read> (input: R, level: u32) -> std::io::Result<Vec<u8>> {
    let outbuf: Vec<u8> = Vec::new();
    let gz = GzBuilder::new ()
        .write(outbuf, Compression::new(level));
    return from_encoder (input, gz);
}

pub fn from_encoder<R: Read, W: Write>(mut input: R, mut gz: GzEncoder<W>) -> std::io::Result<W> {
    let mut inbuf: Vec<u8> = Vec::new();
    input.read_to_end(&mut inbuf)?;
    gz.write_all(inbuf.as_slice())?;
    return gz.finish();
}
