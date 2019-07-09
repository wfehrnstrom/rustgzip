use std::fs;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(linux)]
use std::os::linux::fs::MetadataExt;

use crate::{EXIT_CODE, Opt, constants};

extern crate num;

use std::time::{Duration, SystemTime};
use std::convert::TryInto;

use num::Integer;

pub struct WrappedDir<'a, 'b> {
    pub path: &'a Path,
    pub dir: &'b fs::ReadDir
}

pub struct WrappedFile<'a, 'b> {
    pub path: &'a Path,
    pub file: &'b fs::File,
}

pub struct Timespec (i64, i32);

impl From<Duration> for Timespec  {
    fn from (d: Duration) -> Self {
        let mut secs;
        let nsecs;
        if d.as_secs() > i64::max_value().try_into().unwrap() {
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

impl From<SystemTime> for Timespec {
    fn from (s: SystemTime) -> Self {
        let d = match s.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => dur,
            Err(_) => return Timespec (-1, -1)
        };
        return Timespec::from(d);
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
}

pub fn bit_set<T: Integer + std::ops::BitAnd + PartialEq> (b1: T, b2: T) -> bool
where
    T: std::ops::BitAnd<Output = T>{
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
        let unix_checks = cfg!(unix) || unix_file_checks(f, opt)?;
        // TODO: CHECK LINUX AND OTHERS
        if unix_checks {
            return Ok(true);
        }
        return Ok(false);
    }
    Ok(true)
}

#[cfg(unix)]
fn unix_file_checks (f: &WrappedFile, opt: &Opt) -> std::io::Result<bool> {
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

pub fn get_input_size (stat: &fs::Metadata) -> u64 {
    return stat.len();
}

// A result of Error indicates that the file must be skipped
pub fn make_ofname (f: &PathBuf, opt: &Opt) -> Result<Box<PathBuf>, ()> {
    // TODO: Replicate true behavior of get_suffix, which is too confusing to follow
    let ofname = f;
    let suffix = get_suffix (f, opt);
    match suffix {
        Some(suffix) => {
            // convert .tgz and .taz to .tar
            println!("got suffix: {}", suffix);
            return Ok(Box::new(ofname.with_extension(OsStr::new(suffix.as_str()))));
        },
        None => {
            // when in test or list mode and not recursive, we don't care about having a suffix
            if !opt.recursive && (opt.list || opt.test) {
                return Ok(Box::new(ofname.to_path_buf()));
            }
            if (opt.verbose > 0) || !opt.recursive {
                warn! ("{}: {}: unknown suffix -- ignored", constants::PROGRAM_NAME,
                    f.file_name().unwrap().to_str().unwrap(); constants::WARNING);
                return Err(());
            }
            // if we're compressing, the file we're compressing shouldn't already have a compress
            // extension
            if !opt.decompress && !opt.force {
                if (opt.verbose > 0) && (!opt.recursive || !opt.quiet) {
                    // do not affect exit code here
                    eprintln! ("{}: {} already has suffix -- unchanged",
                        constants::PROGRAM_NAME, f.file_name().unwrap().to_str().unwrap())
                }
                return Err(());
            }
        }
    }
    Ok(Box::new(ofname.with_extension(OsStr::new(opt.suffix.as_str()))))
}

fn get_suffix (p: &PathBuf, opt: &Opt) -> Option<String> {
    if opt.decompress {
        let known_suffixes = vec!(".gz", ".z", ".taz", ".tgz", "-gz", "-z", "_z");
        let ext = match p.extension () {
            Some(e) => e,
            None => OsStr::new("")
        };
        let ext_str = ext.to_str().unwrap();
        // if extension is of the form .??z and we are on a unix system, return default suffix
        if cfg!(unix) && ext.len() == 3 && ext_str.get(2..) == Some("z") {
            eprintln!("{}: extensions of the form .??z are disallowed on unix-based systems.\n\
            reverting to {}",
                constants::PROGRAM_NAME, constants::DEFAULT_SUFFIX);
            return Some(String::from(constants::DEFAULT_SUFFIX));
        }
        // if suffix given in --suffix is allowable
        if known_suffixes.contains(&ext_str) {
            return Some(String::from(ext_str));
        }
        return None;
    }
    else {
        match p.extension () {
            Some(_) => return None,
            None => return Some(String::from(opt.suffix.as_str()))
        }
    }
}
