use crate::formats::zip::Test;
use crate::formats::list::List;
use crate::formats::zip::Zip;
use std::path::PathBuf;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind, Read, Write};
use crate::{util, Opt, zip, unzip};
use std::convert::TryInto;
use chrono::{DateTime, Datelike, Timelike};
use chrono::offset::{Local, TimeZone};
use crate::util::WrappedFile;
use std::str::FromStr;

pub struct GzFile {
    pub path: Option<PathBuf>,
    compression_method: u8,
    mtime: u32,
    stored_filename: Option<String>,
    flag: GzFlags,
    comment: Option<String>,
    os: u8,
    data: Vec<u8>,
    uncompressed_size: u32,
    crc32: u32,
    hcrc16: Option<u16>,
    xfield: Option<(u16, Vec<u8>)>,
    raw: Vec<u8>
}

struct GzFlags {
    pub ftext: bool,
    pub fhcrc: bool,
    pub fextra: bool,
    pub fname: bool,
    pub fcomment: bool
}

impl TryFrom<Vec<u8>> for GzFile {
    type Error = std::io::Error;
    fn try_from (buf: Vec<u8>) -> Result<Self, Self::Error> {
        if buf.len() < GzFile::HEADER_SIZE_USIZE {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid compressed file size"));
        }
        if buf[0] != 31 || buf[1] != 139 {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid compressed file type"));
        }

        let compression_method = buf[2];
        let flag = buf[3];
        let mtime: u32 = util::shift_left(4, &buf[4..8]);
        let os = buf[9];
        let flag = parse_flags(flag);
        let mut pos: usize = 10;
        let mut xfield = None;
        let mut hcrc16: Option<u16> = None;

        if flag.fextra {
            let extra_field_size: u16 = util::shift_left(2, &buf[pos..pos+2])
                                            .try_into().unwrap();
            pos += 2;
            let size: usize = extra_field_size.into();
            let extra_field = &buf[pos..(pos+size)];
            pos += size;
            xfield = Some((extra_field_size, extra_field.to_vec()));
        }

        let stored_filename = get_str_if_set(&mut pos, buf.as_slice(), flag.fname);
        let comment = get_str_if_set(&mut pos, buf.as_slice(), flag.fcomment);

        if flag.fhcrc {
            hcrc16 = Some(util::shift_left(2, &buf[pos..pos+2]).try_into().unwrap());
            pos += 2;
        }

        let data = buf[pos..buf.len()-8].to_vec();
        pos = buf.len() - 8;
        let crc32 = util::shift_left(4, &buf[pos..pos+4]);
        pos += 4;
        let uncompressed_size = util::shift_left(4, &buf[pos..pos+4]);
        // pos += 4;
        Ok(GzFile {
            path: None,
            compression_method,
            mtime,
            stored_filename,
            flag,
            comment,
            os,
            data,
            uncompressed_size,
            crc32,
            hcrc16,
            xfield,
            raw: buf
        })
    }
}

impl TryFrom<WrappedFile<'_, '_>> for GzFile {
    type Error = std::io::Error;
    fn try_from (wf: WrappedFile) -> Result<Self, Self::Error> {
        let mut buf: Vec<u8> = Vec::new();
        let mut f = wf.file;
        f.read_to_end(&mut buf)?;
        let mut gz_file = GzFile::try_from(buf)?;
        gz_file.path = Some(wf.path.to_path_buf());
        Ok(gz_file)
    }
}

impl List for GzFile {
    const HEADER_SIZE: f64 = GzFile::HEADER_SIZE_F64;
    fn list(&self, opt: &Opt) -> (f64, f64) {
        if opt.verbose > 0 {
            let dt = self.modified_on();
            let datestring = format!("{} {}", Self::month(dt.date().month()), dt.date().day());
            let timestring = format!("{}:{:02}", dt.time().hour(), dt.time().minute());
            print!("{:<8}{:<12x}{:<8}{:<8}", "defla", self.crc32, datestring, timestring)
        }
        let compressed_size: u32 = self.raw.len().try_into().unwrap();
        let compressed_size: f64 = Self::bytes_bound_check(compressed_size.try_into().unwrap(), opt, true).unwrap();
        let uncompressed_size: f64 = Self::bytes_bound_check(self.uncompressed_size.try_into().unwrap(), opt, false).unwrap();
        if opt.name {
            match &self.stored_filename {
                Some(s) => println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", self.raw.len(), self.uncompressed_size,
                    Self::calculate_ratio(compressed_size, uncompressed_size), s),
                None => println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", self.raw.len(), self.uncompressed_size,
                    Self::calculate_ratio(compressed_size, uncompressed_size), "???")
            }
        }
        else {
            let uncompressed_filename = if let Some(p) = &self.path {
                let uncompressed_filename = match util::make_ofname(&p, opt) {
                    Ok(boxed_path_buf) => boxed_path_buf,
                    Err(_) => Box::new(PathBuf::from_str("").unwrap())
                };
                String::from((*uncompressed_filename).to_str().unwrap())
            }
            else{
                String::from("????")
            };
            println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", self.raw.len(), self.uncompressed_size,
                Self::calculate_ratio(compressed_size, uncompressed_size), uncompressed_filename)
        }
        return (compressed_size, uncompressed_size)
    }
}

impl Zip for GzFile {
    fn compress<R: Read>(input: R, opt: &Opt) -> Result<Vec<u8>, std::io::Error> {
        zip::from(input, opt.level.try_into().unwrap())
    }

    fn compress_into<R: Read>(input: R, opt: &Opt) -> Result<Box<Self>, std::io::Error> {
        let res = Self::compress(input, opt)?;
        let s = Self::try_from (res)?;
        Ok(Box::new(s))
    }

    fn decompress (self) -> Result<Vec<u8>, std::io::Error> {
        unzip::from(&self.raw[..])
    }
}

impl Test for GzFile {
    fn test (self, opt: &Opt) -> bool {
        match GzFile::decompress(self) {
            Ok(b) => {
                if let Err(_) = std::io::stdout().write_all(b.as_slice()){
                    return false
                }
                if let Err(_) = std::io::stdout().flush() {
                    return false
                }
                if opt.verbose > 0 {
                    println!(" OK");
                }
                true
            }
            Err(_) => false
        }
    }
}

impl GzFile {
    const HEADER_SIZE_F64: f64 = 18.0;
    const HEADER_SIZE_USIZE: usize = 18;

    pub fn modified_on(&self) -> DateTime<Local> {
        Local.timestamp (self.mtime.into(), 0)
    }
}

fn parse_flags (byte: u8) -> GzFlags {
    GzFlags {
        ftext: util::bit_set(byte, 0b0000_0001),
        fhcrc: util::bit_set(byte, 0b0000_0010),
        fextra: util::bit_set(byte, 0b0000_0100),
        fname: util::bit_set(byte, 0b0000_1000),
        fcomment: util::bit_set(byte, 0b0001_0000)
    }
}

fn get_str_if_set  (hpos: &mut usize, buf_slice: & [u8], cond: bool) -> Option<String> {
    if cond {
        if let Ok((s, pos)) = util::str_seek(*hpos, buf_slice) {
            *hpos += pos;
            return Some(String::from(s));
        }
    }
    return None;
}
