use std::io::Read;
use flate2::read::GzDecoder;

pub fn from<R: Read>(input: R) -> std::io::Result<Vec<u8>> {
    let gz = GzDecoder::new (input);
    return from_decoder (gz);
}

pub fn from_decoder<R: Read>(mut gz: GzDecoder<R>) -> std::io::Result<Vec<u8>> {
    let mut outbuf: Vec<u8> = Vec::new();
    gz.read_to_end(&mut outbuf)?;
    return Ok(outbuf);
}
