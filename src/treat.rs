use std::path::{PathBuf, Path};
use std::fs::{File, read_dir, ReadDir, metadata};
use std::io::{Error, ErrorKind};
use std::convert::TryInto;
use crate::{Opt, EXIT_CODE};
use crate::warn;
use crate::util;
use crate::zip;
use crate::unzip;

use crate::constants;

pub fn files (files: Vec<PathBuf>, opt: &mut Opt) {
    for file in files {
        match self::file (file, opt) {
            Ok(()) => continue,
            Err(_) => continue
        }
    }
}

fn file (filepath: PathBuf, opt: &mut Opt) -> std::io::Result<()> {
    let fstr = match filepath.to_str() {
        Some(s) => s,
        None => panic!("{}: file given is not in valid unicode!", constants::PROGRAM_NAME)
    };
    println!("filepath: {}", fstr);
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
            let wrapped_dir = util::WrappedDir {path: fpath, dir: dir};
            self::try_dir(wrapped_dir, opt)?;
        }
        else {
            let wrapped_file = util::WrappedFile {path: fpath, file: &f};
            if !util::check_file_modes(&wrapped_file, opt)? {
                return Err(Error::new(ErrorKind::InvalidData, ""));
            }
            // if this fails, we must have something that isn't a regular file
            let _mtime = match util::get_input_time(&stat) {
                Ok(mtime) => mtime,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, ""))
            };
            let _size = util::get_input_size(&stat);
            let _part_nb = 0;
            let ofname = match util::make_ofname(&filepath, opt) {
                Ok(boxed_path_buf) => boxed_path_buf,
                Err(_) => return Err(std::io::Error::new(ErrorKind::InvalidData, ""))
            };
            let ofname_str: &str = (*ofname).to_str().unwrap();
            if file_would_replace(ofname_str) && !opt.force {
                print!("{}: {} already exists; do you wish to overwrite (y or n)? ",
                    constants::PROGRAM_NAME, ofname_str);
                if util::yesno() {
                    run_compression(&wrapped_file, ofname_str, opt)?;
                }
                else {
                    println!("\tnot overwritten");
                }
            }
            else {
                run_compression(&wrapped_file, ofname_str, opt)?;
            }
        }
        Ok(())
    }
}

fn run_compression (file: &util::WrappedFile, ofname_str: &str, opt: &Opt) -> std::io::Result<()> {
    if !opt.decompress {
        zip::into (&file, ofname_str, opt.level.try_into().unwrap())
    }
    else {
        unzip::into(&file, ofname_str)
    }
}

fn file_would_replace (file_name: &str) -> bool {
    let p: &Path = Path::new(file_name);
    match metadata(p) {
        Ok(_) => true,
        Err(e) => if e.kind() == ErrorKind::AlreadyExists {true} else {false}
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
        let dir_name = dir.path.as_os_str().to_str().unwrap();
        warn!("{}: {}: is a directory -- ignored", constants::PROGRAM_NAME, dir_name; constants::WARNING);
    }
    Ok (())
}

fn dir (dir: ReadDir, opt: &mut Opt) {
    let files: Vec<PathBuf> = dir.filter_map(|f|{
        match f {
            Ok(dir_entry) => {
                match dir_entry.file_type() {
                    Ok(t) => if t.is_file() {Some(dir_entry.path())} else {None},
                    Err(_) => None
                }
            },
            Err(_) => None
        }
    }).collect();
    println!("num files: {}", files.len());
    self::files(files, opt);
}
