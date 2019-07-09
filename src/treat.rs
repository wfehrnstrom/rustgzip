use std::path::{PathBuf, Path};
use std::fs::{File, read_dir, ReadDir};
use std::io::{Error, ErrorKind};
use crate::{Opt, EXIT_CODE};
use crate::warn;
use crate::util;

use crate::constants;

pub fn files (files: Vec<PathBuf>, opt: &mut Opt) {
    for file in files {
        match self::file (file, opt) {
            Ok(()) => continue,
            Err(_) => panic!("{}: operation unsuccessful!", constants::PROGRAM_NAME)
        }
    }
}

fn file (filepath: PathBuf, opt: &mut Opt) -> std::io::Result<()> {
    let fstr = match filepath.to_str() {
        Some(s) => s,
        None => panic!("{}: file given is not in valid unicode!", constants::PROGRAM_NAME)
    };
    if check_for_stdin(fstr, opt) {
        return Ok(());
    }
    else{
        let fpath: &Path = filepath.as_path();
        let f = match File::open(fpath) {
            Ok(file) => file,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotFound => eprintln!("{}: {} not found", constants::PROGRAM_NAME, fstr),
                    ErrorKind::PermissionDenied => eprintln!("{}: permission denied to open {}",
                        constants::PROGRAM_NAME, fstr),
                    _ => eprintln!("{}: unknown error occured while opening {}",
                            constants::PROGRAM_NAME, fstr)
                }
                return Err(e);
            }
        };
        let stat = f.metadata()?;
        if stat.is_dir() {
            let dir: ReadDir = read_dir(fpath)?;
            let wrapped_dir = util::WrappedDir {path: fpath, dir: &dir};
            self::try_dir(wrapped_dir, opt)?;
        }
        else {
            let wrapped_file = util::WrappedFile {path: fpath, file: &f};
            if !util::check_file_modes(&wrapped_file, opt)? {
                return Err(Error::new(ErrorKind::InvalidData, ""));
            }
            // if this fails, we must have something that isn't a regular file
            let mtime = match util::get_input_time(&stat) {
                Ok(mtime) => mtime,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, ""))
            };
            let size = util::get_input_size(&stat);
            let part_nb = 0;
            let ofname = match util::make_ofname(&filepath, opt) {
                Ok(boxed_path_buf) => boxed_path_buf,
                Err(_) => return Err(std::io::Error::new(ErrorKind::InvalidData, ""))
            };
        }
        Ok(())
    }
}

fn check_for_stdin (fstr: &str, opt: &mut Opt) -> bool {
    if fstr == "-" {
        let cflag = opt.stdout;
        self::stdin(opt);
        opt.stdout = cflag;
        return true;
    }
    return false;
}

fn stdin (_opt: &Opt) {
    unimplemented!();
}

fn try_dir (dir: util::WrappedDir, opt: &mut Opt) -> std::io::Result<()> {
    if opt.recursive {
        self::dir(dir.dir, opt);
    }
    else{
        println!("recursive not on");
        warn!("{}: {:?}: is a directory -- ignored", constants::PROGRAM_NAME, dir.path; constants::WARNING);
    }
    Ok (())
}

fn dir (_dir: &ReadDir, _opt: &mut Opt) {
    unimplemented!();
}
