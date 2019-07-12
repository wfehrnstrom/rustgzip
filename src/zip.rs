use std::io::{Read, Write};
use flate2::{Compression, GzBuilder};

pub fn from<R: Read> (mut input: R, level: u32) -> std::io::Result<Vec<u8>> {
    let mut inbuf: Vec<u8> = Vec::new();
    let outbuf: Vec<u8> = Vec::new();
    input.read_to_end(&mut inbuf)?;
    let mut gz = GzBuilder::new ()
                    .write (outbuf, Compression::new(level));
    gz.write_all(inbuf.as_slice())?;
    let outbuf = gz.finish()?;
    return Ok(outbuf);
}
