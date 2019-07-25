use crate::util::WorkData;
use std::io::Read;
use crate::Opt;
use chrono::DateTime;
use chrono::offset::{Local, TimeZone};

pub trait Zip {
    fn decompress (self) -> Result<Vec<u8>, std::io::Error>;
    fn compress <R: Read> (input: R, wdata: Option<WorkData>, opt: &Opt) -> Result<Vec<u8>, std::io::Error>;
    fn compress_into <R: Read> (input: R, wdata: Option<WorkData>, opt: &Opt) -> Result<Box<Self>, std::io::Error>;
}

pub trait Test {
    fn test (self, opt: &Opt) -> bool;
}

pub struct StreamHeader {
    magic: Vec<u8>,
    orig_filename: String,
    crc32: u32,
    compress_len: u32,
    uncompress_len: u32,
    pub mtime: u32
}

pub trait Header {
    fn compr_method (&self) -> String;
    fn get_header (&self) ->  StreamHeader;
    fn modified_on (&self) -> DateTime<Local> {
        Local.timestamp (self.get_header().mtime.into(), 0)
    }
    fn orig_file_name (&self, opt: &Opt) -> String;
}
