use crate::util::{WrappedFile, WorkData};
use std::path::{PathBuf, Path};
use std::fs::{File, ReadDir, remove_file, read_dir, metadata};
use std::io::{Read, Write, Error, ErrorKind};
use std::convert::TryInto;
use std::time::SystemTime;
use std::process::exit;
use std::str::from_utf8;
use crate::{Opt, EXIT_CODE};
use crate::warn;
use crate::util;
use crate::zip;
use crate::unzip;
use crate::constants;
use flate2::{Compression, GzBuilder};
use flate2::read::GzDecoder;

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
    let work_data = WorkData {
        mtime: None,
        orig_name: None,
        ofname: String::from("stdout")
    };
    if let Err(_) = work (std::io::stdin(), work_data, opt) {
        exit (constants::ERROR);
    }
}

fn file (filepath: PathBuf, opt: &mut Opt) -> std::io::Result<()> {
    let fstr = match filepath.to_str() {
        Some(s) => s,
        None => {
            let msg = "file does not have valid unicode name";
            eprintln!("{}: {}", constants::PROGRAM_NAME, msg);
            return Err(Error::new(ErrorKind::InvalidInput, msg))
        }
    };
    if check_for_stdin(fstr) {
        let cflag = opt.stdout;
        self::stdin(opt);
        opt.stdout = cflag;
        return Ok(());
    }
    else{
        let fpath: &Path = filepath.as_path();
        let f = util::file_open (&filepath)?;
        let stat = f.metadata()?;
        if stat.is_dir() {
            let dir: ReadDir = read_dir(fpath)?;
            let wrapped_dir = util::WrappedDir {path: fpath, dir: dir};
            self::try_dir(wrapped_dir, opt)?;
        }
        else {
            let wrapped_file = util::WrappedFile {path: fpath, file: &f};
            // if this fails, we must have something that isn't a regular file
            if !util::check_file_modes(&wrapped_file, opt)? {
                return Err(Error::new(ErrorKind::InvalidData, ""));
            }

            // let _size = util::get_input_size(&stat);

            let ofname = match util::make_ofname(&filepath, opt) {
                Ok(boxed_path_buf) => boxed_path_buf,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Error constructing
                    new output file name"))
            };

            let ofname_str: &str = (*ofname).to_str().unwrap();

            let mtime = match util::get_input_time(&stat) {
                Ok(mtime) => mtime,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, ""))
            };

            let work_data = WorkData::new (Some(String::from(fstr)), Some(mtime), String::from(ofname_str), opt);

            if file_would_replace(ofname_str) && !opt.force && !opt.stdout {
                overwrite_prompt(&wrapped_file, work_data, opt)?;
            }
            else {
                work(wrapped_file.file, work_data, opt)?;
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

fn work<R: Read> (input: R, work_data: WorkData, opt: &mut Opt) -> std::io::Result<()> {
    let ofname_str = work_data.ofname;
    let mut name_from_compressed_file: Option<String> = None;
    let mut mtime_from_compressed_file: Option<u32> = None;
    let output: Vec<u8> = if !opt.decompress {
        if opt.no_name || work_data.mtime.is_none() || work_data.orig_name.is_none() {
            zip::from (input, opt.level.try_into().unwrap())?
        }
        else {
            let gz = GzBuilder::new()
                            .filename(work_data.orig_name.unwrap().as_str())
                            .mtime(work_data.mtime.unwrap().try_into().unwrap())
                            .write(Vec::new(), Compression::new(opt.level.try_into().unwrap()));
            zip::from_encoder(input, gz)?
        }
    }
    else {
        if opt.no_name {
            unzip::from(input)?
        }
        else {
            let gz = GzDecoder::new (input);
            let gz_header = gz.header();
            if let Some(h) = gz_header {
                if let Some(filename) = h.filename() {
                    name_from_compressed_file = Some(String::from(from_utf8(filename).unwrap()));
                }
                mtime_from_compressed_file = Some(h.mtime());
            }
            unzip::from_decoder(gz)?
        }
    };

    if opt.stdout || ofname_str == "stdout" {
        std::io::stdout().write_all(output.as_slice())?;
        std::io::stdout().flush()?;
    }
    else {
        let mut fname = ofname_str.clone();
        // if we are compressing or we specified no_name on decompression, calculate the output
        // file name
        let mut f = if opt.no_name || !opt.decompress {
            File::create(ofname_str)?
        }
        // otherwise use the one stored within the file, falling back to the calculated
        // if necessary
        else {
            if let Some(name) = name_from_compressed_file {
                fname = name.clone();
                File::create(name)?
            }
            else{
                File::create(ofname_str)?
            }
        };
        f.write_all(output.as_slice())?;
        f.flush()?;
        // modify the mtime as well if we have that
        if !opt.no_name && opt.decompress {
            if let Some(mtime) = mtime_from_compressed_file {
                utime::set_file_times(fname, SystemTime::now().duration_since(
                    SystemTime::UNIX_EPOCH).unwrap().as_secs(), mtime.into())?;
            }
        }
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

fn overwrite_prompt (wrapped_file: &WrappedFile, work_data: WorkData, opt: &mut Opt) -> std::io::Result<()> {
    print!("{}: {} already exists; do you wish to overwrite (y or n)? ",
        constants::PROGRAM_NAME, &work_data.ofname);
    if util::yesno() {
        work(wrapped_file.file, work_data, opt)
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

pub mod errors {
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
        eprintln!("{0}: compressed data not {1} a terminal. \
		  Use -f to force {2}compression.\n\
		  For help, type: {0} -h", super::constants::PROGRAM_NAME,
            readwrite, de);
    }
}
