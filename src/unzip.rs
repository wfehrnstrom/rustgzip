use std::io::Read;
use flate2::read::GzDecoder;

pub fn from<R: Read>(input: R) -> std::io::Result<Vec<u8>>{
    let mut outbuf: Vec<u8> = Vec::new();
    let mut gz = GzDecoder::new (input);
    gz.read_to_end(&mut outbuf)?;
    return Ok(outbuf);
}
