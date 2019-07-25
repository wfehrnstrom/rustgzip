use crate::util::WorkData;
use crate::formats::zip::Test;
use crate::formats::list::List;
use crate::formats::zip::Zip;
use std::path::PathBuf;
use std::convert::{TryFrom, TryInto};
use std::io::{Error, ErrorKind, Read, Write};
use crate::{util, Opt, zip};
use chrono::DateTime;
use chrono::offset::{Local, TimeZone};
use crate::util::WrappedFile;
use flate2::{GzBuilder, Compression};
use flate2::read::MultiGzDecoder;

#[derive(Debug)]
pub struct GzFile {
    pub path: Option<PathBuf>,
    compression_method: u8,
    pub mtime: u32,
    pub stored_filename: Option<String>,
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

#[derive(Debug)]
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
        if !Self::is_magic_num(&buf[0..2]) {
            println!("uh oh, not a .gz-file");
            return Err(Error::new(ErrorKind::InvalidInput, "invalid compressed file type"));
        }

        let compression_method = buf[2];
        let flag = buf[3];
        let mtime: u32 = util::shift_left(4, &buf[4..8]);
        let os = buf[9];
        let flag = GzFile::parse_flags(flag);
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

        let stored_filename = util::get_str_if_set(&mut pos, buf.as_slice(), flag.fname);
        let comment = util::get_str_if_set(&mut pos, buf.as_slice(), flag.fcomment);

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

impl TryFrom<WrappedFile<'_, '_>> for GzFile
    {
    type Error = std::io::Error;
    fn try_from (wf: WrappedFile) -> Result<Self, Self::Error> {
        let mut buf: Vec<u8> = Vec::new();
        let mut f = wf.file;
        f.read_to_end(&mut buf)?;
        let mut gz_file: GzFile = TryFrom::try_from(buf)?;
        gz_file.path = Some(wf.path.to_path_buf());
        Ok(gz_file)
    }
}

impl List for GzFile {

    fn header_size(&self) -> f64 {
        return 18.0;
    }

    fn list(&self, opt: &Opt) -> (f64, f64) {
        if opt.verbose > 0 {
            let dt = self.modified_on();
            print!("{:<8}{:<12x}{:<8}{:<8}", "defla", self.crc32, Self::datestring(&dt), Self::timestring(&dt))
        }
        let uncompressed_filename: String = Self::get_filename_str (&self.stored_filename, &self.path, opt);
        let compressed_size: u32 = self.raw.len().try_into().unwrap();
        let compressed_size: f64 = Self::bytes_bound_check(compressed_size.try_into().unwrap(), opt, true, self.header_size()).unwrap();
        let uncompressed_size: f64 = Self::bytes_bound_check(self.uncompressed_size.try_into().unwrap(), opt, false, self.header_size()).unwrap();
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", compressed_size, self.uncompressed_size,
            Self::calculate_ratio(compressed_size, Some(uncompressed_size), self.header_size()), uncompressed_filename);
        return (compressed_size, uncompressed_size)
    }
}

impl Zip for GzFile {
    fn compress<R: Read>(input: R, wdata: Option<WorkData>, opt: &Opt) -> Result<Vec<u8>, std::io::Error> {
        let os = GzFile::os();
        let gz = match wdata {
            Some(wdata) => GzBuilder::new()
                            .filename(wdata.orig_name.unwrap().as_str())
                            .mtime(wdata.mtime.unwrap().try_into().unwrap())
                            .operating_system(os)
                            .write(Vec::new(), Compression::new(opt.level.try_into().unwrap())),
            None => GzBuilder::new()
                        .operating_system(os)
                        .write(Vec::new(), Compression::new(opt.level.try_into().unwrap()))
        };
        let compressed = zip::from_encoder(input, gz)?;
        Ok(compressed)
    }

    fn compress_into<R: Read>(input: R, wdata: Option<WorkData>, opt: &Opt) -> Result<Box<Self>, std::io::Error> {
        let res = Self::compress(input, wdata, opt)?;
        let s = std::convert::TryFrom::try_from (res)?;
        Ok(Box::new(s))
    }

    fn decompress (self) -> Result<Vec<u8>, std::io::Error> {
        let mut gz = MultiGzDecoder::new(&self.raw[..]);
        let mut outbuf: Vec<u8> = Vec::new();
        gz.read_to_end(&mut outbuf)?;
        Ok(outbuf)
    }
}

impl Test for GzFile {
    fn test (self, opt: &Opt) -> bool {
        fn err(opt: &Opt) -> bool {
            if opt.verbose > 0 {
                println!(" CORRUPTED");
            }
            return false
        }

        match GzFile::decompress(self) {
            Ok(b) => {
                if let Err(_) = std::io::stdout().write_all(b.as_slice()){
                    return err(opt);
                }
                if let Err(_) = std::io::stdout().flush() {
                    return err(opt);
                }
                if opt.verbose > 0 {
                    println!(" OK");
                }
                true
            }
            Err(_) => {
                return err(opt);
            }
        }
    }
}

impl GzFile {
    const HEADER_SIZE_USIZE: usize = 18;

    pub fn is_magic_num (bytes: &[u8]) -> bool {
        return bytes[0] == 31 && bytes[1] == 139
    }

    pub fn modified_on(&self) -> DateTime<Local> {
        Local.timestamp (self.mtime.into(), 0)
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

    pub fn os () -> u8 {
        #[cfg(target_os = "windows")]
        return 0;
        #[cfg(any(target_os = "ios", target_os = "macos", target_os = "linux"))]
        return 3;
        #[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "linux", target_os = "windows")))]
        return 255;
    }
}
