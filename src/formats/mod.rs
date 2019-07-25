use crate::Opt;
use crate::formats::list::List;
use std::convert::{TryFrom, TryInto};
use gz::GzFile;
// use pkzip::ZipFile;
use crate::util::WrappedFile;
use std::io::{Read, ErrorKind};

pub mod gz;
pub mod list;
pub mod zip;
// pub mod pkzip;

pub fn parse_list (mut wf: WrappedFile) -> Box<dyn List> {
    let mut buf: Vec<u8> = Vec::new();
    wf.file.read_to_end(&mut buf).unwrap();
    let buf_len: usize = buf.len();
    let magic_portion = buf[0..4].to_vec();
    let path = wf.path;
    let buf: Vec<u8> = Vec::new();
    // ... here read the file into the buf
    if GzFile::is_magic_num(&magic_portion) {
        if let Ok(f) = TryFrom::try_from(buf) {
            let mut f: GzFile = f;
            f.path = Some(path.to_path_buf());
            return Box::new(f);
        }
    }
    // else if ZipFile::is_magic_num(&magic_portion){
    //     if let Ok(f) = TryFrom::try_from(buf.as_slice()) {
    //         let f: ZipFile = f;
    //         return Box::new(f);
    //     }
    // }
    let f = UnknownFile::from(buf_len);
    return Box::new(f);
}

pub struct UnknownFile {
    compressed_size: u32
}

impl From<usize> for UnknownFile {
    fn from (len: usize) -> Self {
        UnknownFile { compressed_size: match len.try_into() {
            Ok(n) => n,
            Err(_) => u32::max_value()
        }}
    }
}

impl List for UnknownFile {
    fn header_size (&self) -> f64 {
        return 0.0;
    }

    fn list (&self, opt: &Opt) -> (f64, f64) {
        if opt.verbose > 0 {
            print!("{:<8}{:<12}{:<8}{:<8}", "????", "????????", "????", "??:??");
        }
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", self.compressed_size, "??", 0.0, "????????");
        return (self.compressed_size.into(), self.compressed_size.into());
    }
}

pub trait TryFromReadable<R>
    where R: Read, Self: Sized {
    fn try_from (buf: R) -> Result<Self, std::io::Error>;
}

impl<'a, R: Read, U: TryFrom<Vec<u8>>> TryFromReadable<R> for U {
    fn try_from (mut buf: R) -> Result<Self, std::io::Error> {
        let mut v: Vec<u8> = Vec::new();
        match buf.read_to_end(&mut v) {
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(ErrorKind::InvalidData, ""))
        }
        return match Self::try_from(v){
            Ok(s) => Ok(s),
            Err(_) => return Err(std::io::Error::new(ErrorKind::InvalidData, ""))
        }
    }
}
