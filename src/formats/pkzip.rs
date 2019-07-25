extern crate zip;

use crate::util::WorkData;
use crate::Opt;
use crate::formats::zip::Zip;
use crate::formats::list::List;
use std::io::{Read, Write};
use std::io;
use std::convert::{TryFrom, TryInto};
use chrono::{NaiveDateTime, Datelike, Timelike};
use zip::DateTime;

pub struct ZipFile<'a> {
    file: zip::read::ZipFile<'a>
}

impl Zip for ZipFile<'_> {
    fn decompress (mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut v: Vec<u8> = Vec::new();
        self.file.read_to_end(&mut v);
        Ok(v)
    }

    fn compress<R: Read> (mut input: R, wdata: Option<WorkData>, opt: &Opt) -> Result<Vec<u8>, io::Error> {
        let mut inbuf: Vec<u8> = Vec::new();
        input.read_to_end(&mut inbuf);
        let mut name: std::string::String = String::from("thing");
        let opts = match wdata {
            Some(data) => {
                if let Some(orig_name) = data.orig_name {
                    name = orig_name;
                }
                match data.mtime {
                    Some(time) => {
                        let time = NaiveDateTime::from_timestamp(time.0, time.1.try_into().unwrap());
                        let year: u16 = time.year().try_into().unwrap();
                        let boundedYear: u16 = if year < 1980 {
                            1980
                        } else if year > 2107 {
                            2107
                        }
                        else {
                            year
                        };
                        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated)
                            .last_modified_time(
                                DateTime::from_date_and_time(
                                    boundedYear,
                                    time.month().try_into().unwrap(),
                                    time.day().try_into().unwrap(),
                                    time.hour().try_into().unwrap(),
                                    time.minute().try_into().unwrap(),
                                    time.second().try_into().unwrap()
                                ).unwrap())
                            .compression_method(zip::CompressionMethod::Deflated)
                    },
                    None => zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated)
                }
            },
            None => zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated)
        };
        let buf: Vec<u8> = Vec::new();
        let outbuf = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(outbuf);
        zip.start_file(name, opts)?;
        zip.write(&inbuf);
        match zip.finish() {
            Ok(seekable_buf) => Ok(seekable_buf.into_inner()),
            Err(_) => Err(io::Error::new(io::ErrorKind::InvalidData, "pkzip compression error"))
        }
    }

    fn compress_into<R: Read> (input: R, wdata: Option<WorkData>, opt: &Opt) -> Result<Box<Self>, io::Error> {
        let mut compressed = Self::compress (input, wdata, opt)?;
        let mut slice = &compressed[..];
        match zip::read::read_zipfile_from_stream(&mut slice) {
            Ok(opt) => match opt {
                Some(file) => Ok(Box::new(ZipFile {file})),
                None => Err(io::Error::new(io::ErrorKind::InvalidData, "pkzip compression error"))
            },
            Err(_) => Err(io::Error::new(io::ErrorKind::InvalidData, "pkzip compression error"))
        }
    }
}

impl <'a> TryFrom<&'a [u8]> for ZipFile<'a> {
    type Error = std::io::Error;
    fn try_from (buf: &'a [u8]) -> Result<Self, Self::Error> {
        match zip::read::read_zipfile_from_stream(&buf) {
            Ok(opt_zfile) => match opt_zfile {
                Some(zfile) => Ok(ZipFile {file: zfile}),
                None => Err(Self::Error::new(io::ErrorKind::InvalidData, "could not read zip file from stream"))
            },
            Err(_) => Err(Self::Error::new(io::ErrorKind::InvalidData, "could not read zip file from stream"))
        }
    }
}

impl List for ZipFile<'_> {
    fn list(&self, opt: &Opt) -> (f64, f64) {
        let datetime = self.file.last_modified();
        let datestring = Self::datestring_raw (datetime.month (), datetime.day ());
        let timestring = Self::timestring_raw (datetime.hour (), datetime.minute ());
        if opt.verbose > 0 {
            print!("{:<8}{:<12x}{:<8}{:<8}", "defla", self.file.crc32(), datestring, timestring);
        }
        let compr_size: u32 = self.file.compressed_size().try_into().unwrap();
        let uncompr_size: u32 = self.file.size().try_into().unwrap();
        let ratio = Self::calculate_ratio (compr_size.try_into().unwrap(),
            Some(uncompr_size.try_into().unwrap()), self.header_size());
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", self.file.compressed_size(),
            self.file.size(), ratio, self.file.name());
        (compr_size.try_into().unwrap(), uncompr_size.try_into().unwrap())
    }

    fn header_size(&self) -> f64 {
        return 30.0;
    }
}

impl ZipFile<'_> {
    pub fn is_magic_num (bytes: &[u8]) -> bool {
        return (bytes[0] == 0x4b) && (bytes[1] == 0x50) && (bytes[2] == 0x03) && (bytes[3] == 0x04)
    }
}
