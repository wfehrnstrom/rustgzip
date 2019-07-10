use std::fs::{read, File};
use std::io::Write;
use crate::util::WrappedFile;
use flate2::{Compression, GzBuilder};

pub fn into (input_file: &WrappedFile, ofname: &str) -> std::io::Result<()> {
    let bytes_read = read (input_file.path)?;
    let output_file = File::create (ofname)?;
    let mut gz = GzBuilder::new ()
                    .filename (ofname)
                    .write (output_file, Compression::default());
    gz.write_all(bytes_read.as_slice())?;
    gz.finish()?;
    return Ok(());
}
