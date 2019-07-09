use std::path::{PathBuf, Path};
use std::fs::{File, read_dir, ReadDir};
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
        None => panic!("{}: File given is not in valid unicode!", constants::PROGRAM_NAME)
    };
    if fstr == "-" {
        let cflag = opt.stdout;
        self::stdin(opt);
        opt.stdout = cflag;
        Ok(())
    }
    else{
        let fpath: &Path = filepath.as_path();
        let f = File::open(fpath)?;
        let stat = f.metadata()?;
        if stat.is_dir() {
            let dir: ReadDir = read_dir(fpath)?;
            let wrapped_dir = util::WrappedDir {path: fpath, dir: &dir};
            self::try_dir(wrapped_dir, opt)?;
        }
        else {
            let wrapped_file = util::WrappedFile {path: fpath, file: &f};
            if util::check_file_modes(&wrapped_file, opt)? {
                println!("file modes good!");
                unimplemented!();
            }
        }
        Ok(())
    }
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
