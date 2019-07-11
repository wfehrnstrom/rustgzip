use crate::util::WrappedFile;
use std::io::{Read, Write};
use std::fs::File;
use flate2::read::GzDecoder;

pub fn into(input_file: &WrappedFile, ofname: &str) -> std::io::Result<()>{
    println!("decompressing: {}", ofname);
    let input_file = input_file.file;
    let mut output_file = File::create (ofname)?;
    let mut gz = GzDecoder::new (input_file);
    let mut buf = String::new ();
    gz.read_to_string(&mut buf)?;
    output_file.write_all (buf.as_bytes())?;
    return Ok(());
}
