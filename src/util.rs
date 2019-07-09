use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(linux)]
use std::os::linux::fs::MetadataExt;

use crate::{EXIT_CODE, Opt, constants};

extern crate num;

use num::Integer;

pub struct WrappedDir<'a, 'b> {pub path: &'a Path, pub dir: &'b fs::ReadDir}

pub struct WrappedFile<'a, 'b> {pub path: &'a Path, pub file: &'b fs::File}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bit_test(){
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
        let unix_checks = cfg!(not(target_os = "unix")) || unix_file_checks(f, opt)?;
        // TODO: CHECK LINUX AND OTHERS
        if unix_file_checks(f, opt)? {
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
