use std::fs::File;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{ErrorKind, Write};
use std::io;
use crate::treat::errors;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(linux)]
use std::os::linux::fs::MetadataExt;

use crate::{EXIT_CODE, Opt, constants};

extern crate num;

use std::time::{Duration, SystemTime};
use std::convert::TryInto;

use num::Integer;

/// Convenience struct created to encapsulate both the location of a directory
/// as well as the directory itself
pub struct WrappedDir<'a> {
    pub path: &'a Path,
    pub dir: fs::ReadDir
}

/// Convenience struct created to encapsulate both the location of a file
/// as well as the file itself
pub struct WrappedFile<'a, 'b> {
    pub path: &'a Path,
    pub file: &'b fs::File,
}

pub struct WorkData {
    pub orig_name: Option<String>,
    pub mtime: Option<Timespec>,
    pub ofname: String
}

impl WorkData {
    pub fn new (orig_name: Option<String>, time: Option<Timespec>, ofname: String, opt: &Opt) -> Self {
        let mtime;
        // we might be restoring mtime on gunzipping, but we won't be using the mtime from work
        // data, we'll be using the mtime, if any, stored in the compressed file
        if opt.decompress {
            mtime = None
        }
        else{
            mtime = time
        }
        WorkData {orig_name, mtime, ofname}
    }
}

/// Timespec is (i64, i32) because it must represent negative numbers on no timestamp being
/// recoverable
#[derive(Debug, Copy, Clone)]
pub struct Timespec (i64, i32);

impl From<Duration> for Timespec  {
    fn from (d: Duration) -> Self {
        let mut secs;
        let nsecs;
        if d.as_secs() > i32::max_value().try_into().unwrap() {
            secs = i64::max_value ();
            nsecs = i32::max_value ();
        }
        else{
            secs = d.as_secs().try_into().unwrap();
            if d.subsec_nanos() > i32::max_value().try_into().unwrap() {
                secs += 1;
                nsecs = 0;
            }
            else {
                nsecs = d.subsec_nanos().try_into().unwrap();
            }
        }
        Timespec (secs, nsecs)
    }
}

impl From<u32> for Timespec {
    fn from (secs: u32) -> Self {
        let secs: i64 = secs.into();
        let nanosecs: i32 = 0;
        return Timespec (secs, nanosecs);
    }
}

impl TryInto<u32> for Timespec{
    type Error = std::num::TryFromIntError;
    fn try_into (self) -> Result<u32, Self::Error> {
        return self.0.try_into();
    }
}

impl From<SystemTime> for Timespec {
    fn from (s: SystemTime) -> Self {
        match s.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => return Timespec::from(dur),
            Err(_) => return Timespec (-1, -1)
        }
    }
}

static KNOWN_SUFFIXES: [&str; 7] = [constants::DEFAULT_SUFFIX, "z", "taz", "tgz", "-gz", "-z", "_z"];

/// checks if anded together, the two types yield a non-zero value
pub fn bit_set<T: Integer + std::ops::BitAnd + PartialEq> (b1: T, b2: T) -> bool
where
    T: std::ops::BitAnd<Output = T>
{
    let res: T = b1 & b2;
    return res != num::zero();
}

// format of warn! = (eprintln stuff; exit_code)
#[macro_export]
macro_rules! warn {
    ($($arg:expr),*; $exit_code:expr) => {{
        unsafe {EXIT_CODE = $exit_code;}
        eprintln!($($arg),*);
    }}
}

pub fn check_file_modes (f: &WrappedFile, opt: &mut Opt) -> std::io::Result<bool> {
    if !opt.stdout {
        let stat = f.file.metadata()?;
        let file_type = stat.file_type();
        if !file_type.is_file() {
            warn!("{}: {} is not a directory or a regular file - ignored",
                constants::PROGRAM_NAME, f.path.to_str().unwrap(); constants::WARNING);
            return Ok(false)
        }
        let file_checks = (!cfg!(unix) && !cfg!(linux)) || file_checks(f, opt)?;
        if file_checks {
            return Ok(true);
        }
        return Ok(false);
    }
    Ok(true)
}

// TODO: Support linux
fn file_checks (f: &WrappedFile, opt: &Opt) -> std::io::Result<bool> {
    let meta = f.file.metadata()?;
    let mode = meta.mode();

    let path_str = f.path.to_str().unwrap();

    if bit_set (mode, constants::S_ISUID) {
        warn!("{}: {} is set-user-ID on execution - ignored",
            constants::PROGRAM_NAME, path_str; constants::WARNING);
        return Ok(false);
    }
    if bit_set (mode, constants::S_ISGID) {
        warn!("{}: {} is set-group-ID on execution - ignored",
            constants::PROGRAM_NAME, path_str; constants::WARNING);
        return Ok(false);
    }
    if !opt.force {
        if bit_set (mode, constants::S_ISVTX) {
            warn!("{}: {} has the sticky bit set - file ignored",
                constants::PROGRAM_NAME, path_str; constants::WARNING);
            return Ok(false);
        }
        let nlinks = meta.nlink();
        if nlinks > 1 {
            warn!("{}: {} has {} other links -- unchanged",
                constants::PROGRAM_NAME, path_str, nlinks; constants::WARNING);
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn get_input_time (stat: &fs::Metadata) -> std::io::Result<Timespec> {
    if stat.file_type().is_file() {
        return Ok(Timespec::from (stat.modified().unwrap()));
    }
    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Input given is not a file"));
}

pub fn _get_input_size (stat: &fs::Metadata) -> u64 {
    return stat.len();
}

// A result of Error indicates that the file must be skipped
pub fn make_ofname (f: &PathBuf, opt: &Opt) -> Result<Box<PathBuf>, ()> {
    match get_suffix (f, opt) {
        Some(suffix) => {
            let path_str = f.to_str().unwrap();
            if opt.decompress {
                // strip known extension off of end of file
                let res: Vec<&str> = path_str.rsplit('.').skip(1).collect();
                let res: Vec<&str> = res.iter().rev().map(|&x| x).collect();
                let res: String = res.join(".");
                Ok(Box::new(PathBuf::from(res)))
            }
            else {
                // add known extension at end of file
                let res = format!("{}.{}", path_str, suffix);
                Ok(Box::new(PathBuf::from(res)))
            }
        },
        None => Err(())
    }
}

/// Returns the very last portion of the filename (after the last '.'), or None if there was an error
fn get_suffix (p: &PathBuf, opt: &Opt) -> Option<String> {
    if opt.decompress {
        // if the suffix given through the CLI is empty, this means that we MUST try to decompress
        // with whatever suffix is at the end of the file (if any).
        match p.extension() {
            Some(os_str) => {
                let suffix = strip_leading_dot(os_str.to_str().unwrap());
                if opt.suffix.is_empty () || suffix_known(suffix) {
                    return Some(String::from(suffix));
                }
                unknown_suffix_warning(get_file_name(p), opt);
                return None;
            },
            None => {
                if opt.suffix.is_empty () {
                    return Some(String::from (""));
                }
                unknown_suffix_warning(get_file_name(p), opt);
                return None;
            }
        }
    }
    else {
        let has_compression_suffix = match p.extension() {
            Some(os_str) => {
                let suffix = strip_leading_dot(os_str.to_str().unwrap());
                suffix_known (suffix)
            }
            None => false
        };
        if has_compression_suffix && !opt.force {
            let suffix = p.extension().unwrap().to_str().unwrap();
            let file_name = get_file_name(p);
            eprintln!("{}: {} already has .{} suffix -- unchanged", constants::PROGRAM_NAME,
                file_name, suffix);
            return None;
        }
        return Some(String::from(&opt.suffix));
    }
}

fn suffix_known (suffix: &str) -> bool {
    return KNOWN_SUFFIXES.contains (&suffix);
}

pub fn file_open (fpath: &PathBuf) -> std::io::Result<File> {
    let fstr = fpath.to_str().unwrap();
    match File::open(fpath) {
        Ok(file) => Ok(file),
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => eprintln!("{}: {}: No such file or directory", constants::PROGRAM_NAME, fstr),
                ErrorKind::PermissionDenied => eprintln!("{}: {}: permission denied",
                    constants::PROGRAM_NAME, fstr),
                _ => errors::permission_denied_err_msg(fstr, "open")
            }
            return Err(e);
        }
    }
}

fn get_file_name (p: &PathBuf) -> &str {
    match p.file_name() {
        Some(name) => name.to_str().unwrap(),
        None => ".."
    }
}

fn unknown_suffix_warning (filename: &str, opt: &Opt) {
    if (opt.verbose > 0) || (!opt.recursive && !opt.quiet) {
        eprintln!("{}: {}: unknown suffix -- ignored", constants::PROGRAM_NAME,
            filename);
    }
}

pub fn strip_leading_dot (suff: &str) -> &str {
    if let Some(first_char) = suff.chars().next() {
        if first_char == '.' {
            return &suff [1..];
        }
    }
    return suff;
}

pub fn yesno () -> bool {
    io::stdout().flush().unwrap();
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            match input.chars().next() {
                Some(c) => (c == 'y' || c == 'Y'),
                None => false
            }
        },
        Err(_) => false
    }
}

/// 'from' must store its bytes in little endian ordering for its representations
/// to be correct
pub fn shift_left (num_bytes: usize, from: &[u8]) -> u32 {
    if num_bytes > from.len() {
        return 0;
    }
    let mut res: u32 = 0;
    for i in 0..num_bytes {
        let byte: u32 = from[i].into();
        res = res | (byte << (i * 8));
    }
    return res;
}

pub fn str_seek (start: usize, buf: &[u8]) -> Result<(&str, usize), ()> {
    match buf.iter().skip(start).position(|&x| x == 0) {
        Some(pos) => {
            let slice: &[u8] = &buf[start..(start + pos+1)];
            match std::str::from_utf8(slice) {
                Ok(s) => Ok((s, pos+1)),
                Err(_) => Err(())
            }
        },
        None => Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bit_set_test(){
        assert!(bit_set(0b100, 0b100));
        assert!(!bit_set(0b010, 0b100));
        assert!(!bit_set(0b110, 0b001));
        assert!(!bit_set(0b0, 0b0));
    }

    #[test]
    fn strip_leading_dot_test(){
        assert_eq!(strip_leading_dot(".gz"), "gz");
        assert_eq!(strip_leading_dot(".txt.gz"), "txt.gz");
        assert_eq!(strip_leading_dot("gz"), "gz");
        assert_eq!(strip_leading_dot("g.z"), "g.z");
    }

    #[test]
    fn suffix_known_test(){
        assert!(suffix_known("gz"));
        assert!(suffix_known("-z"));
        assert!(!suffix_known("tz"));
    }
}
