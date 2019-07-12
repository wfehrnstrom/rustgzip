use crate::util::WrappedFile;
use std::path::{PathBuf, Path};
use std::fs::{File, ReadDir, remove_file, read_dir, metadata};
use std::io::{Read, Write, Error, ErrorKind};
use std::convert::TryInto;
use std::process::exit;
use crate::{Opt, EXIT_CODE};
use crate::warn;
use crate::util;
use crate::zip;
use crate::unzip;

use crate::constants;

extern crate atty;

pub fn files (files: Vec<PathBuf>, opt: &mut Opt) {
    for file in files {
        match self::file (file, opt) {
            Ok(()) => continue,
            Err(_) => continue
        }
    }
}

pub fn stdin (opt: &mut Opt) {
    let isatty = if opt.decompress {
        atty::is(atty::Stream::Stdin)
    }
    else {
        atty::is(atty::Stream::Stdout)
    };
    if !opt.force && !opt.list && isatty {
        if !opt.quiet {
            errors::tty_err_msg(opt.decompress);
        }
        exit (constants::ERROR);
    }
    if let Err(_) = work (std::io::stdin(), "stdout", opt) {
        exit (constants::ERROR);
    }
}

fn file (filepath: PathBuf, opt: &mut Opt) -> std::io::Result<()> {
    let fstr = match filepath.to_str() {
        Some(s) => s,
        None => panic!("{}: file given is not in valid unicode!", constants::PROGRAM_NAME)
    };
    if check_for_stdin(fstr) {
        let cflag = opt.stdout;
        self::stdin(opt);
        opt.stdout = cflag;
        return Ok(());
    }
    else{
        let fpath: &Path = filepath.as_path();
        let f = file_open (&filepath)?;
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
            let ofname = match util::make_ofname(&filepath, opt) {
                Ok(boxed_path_buf) => boxed_path_buf,
                Err(_) => return Err(std::io::Error::new(ErrorKind::InvalidData, ""))
            };

            let ofname_str: &str = (*ofname).to_str().unwrap();
            if file_would_replace(ofname_str) && !opt.force && !opt.stdout {
                overwrite_prompt(&wrapped_file, ofname_str, opt)?;
            }
            else {
                work(wrapped_file.file, ofname_str, opt)?;
            }

            // delete the file if necessary
            if !opt.stdout && !opt.keep {
                if let Err(e) = remove_file(fpath) {
                    match e.kind() {
                        ErrorKind::PermissionDenied => errors::permission_denied_err_msg(fstr, "delete"),
                        _ => errors::file_delete_err_msg(fstr)
                    }
                }
            }
         }
        Ok(())
    }
}

fn file_open (fpath: &PathBuf) -> std::io::Result<File> {
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

fn work<R: Read> (input: R, ofname_str: &str, opt: &mut Opt) -> std::io::Result<()> {
    let output = if !opt.decompress {
        zip::from (input, opt.level.try_into().unwrap())
    }
    else {
        unzip::from(input)
    }?;

    if opt.stdout || ofname_str == "stdout" {
        std::io::stdout().write_all(output.as_slice())?;
        std::io::stdout().flush()?;
    }
    else {
        let mut f = File::create(ofname_str)?;
        f.write_all(output.as_slice())?;
        f.flush()?;
    }
    Ok(())
}

fn file_would_replace (file_name: &str) -> bool {
    let p: &Path = Path::new(file_name);
    match metadata(p) {
        Ok(_) => true,
        Err(e) => if e.kind() == ErrorKind::AlreadyExists {true} else {false}
    }
}

fn overwrite_prompt (wrapped_file: &WrappedFile, ofname_str: &str, opt: &mut Opt) -> std::io::Result<()> {
    print!("{}: {} already exists; do you wish to overwrite (y or n)? ",
        constants::PROGRAM_NAME, ofname_str);
    if util::yesno() {
        work(wrapped_file.file, ofname_str, opt)
    }
    else {
        println!("\tnot overwritten");
        Ok(())
    }
}

fn check_for_stdin (fstr: &str) -> bool {
    if fstr == "-" {
        return true;
    }
    return false;
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
    self::files(files, opt);
}

mod errors {
    pub fn permission_denied_err_msg (fstr: &str, op: &str) {
        eprintln!("{}: {}: permission denied on {}",
                super::constants::PROGRAM_NAME, fstr, op);
    }

    pub fn file_delete_err_msg (fstr: &str) {
        eprintln!("{}: {}: unexpected error while deleting file.",
            super::constants::PROGRAM_NAME, fstr);
    }

    pub fn tty_err_msg (decompress: bool) {
        let readwrite = if decompress {
            "read from"
        }
        else{
            "written to"
        };
        let de = if decompress {
            "de"
        }
        else {
            ""
        };
        eprintln!("{}: compressed data not {} a terminal. \
		  Use -f to force {}compression.\n\
		  For help, type: {} -h", super::constants::PROGRAM_NAME,
            readwrite, de, super::constants::PROGRAM_NAME);
    }
}
