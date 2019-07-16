extern crate libc;

extern crate clap;

extern crate structopt;

mod constants;
mod treat;
mod util;
mod zip;
mod unzip;
mod list;

use std::path::PathBuf;
use structopt::StructOpt;
use std::process::exit;

static mut EXIT_CODE: i8 = 0;

#[derive(Debug, StructOpt)]
#[structopt(name = "rustzip", about="GNU gzip ported to Rust; aka rustzip.", author="Will Fehrnstrom, wfehrnstrom@gmail.com")]
/// Opt is used to store all the arguments passed through the command line
pub struct Opt {
    #[structopt(short="a", long, help="ascii text; convert end-of-line using local conventions")]
    ascii: bool,
    #[structopt(short="c", long, alias="to-stdout", help="write on standard output, keep original files unchanged")]
    stdout: bool,
    #[structopt(short, long, alias="uncompress", help="decompress")]
    decompress: bool,
    #[structopt(short, long, help="force overwrite of output file and compress links")]
    force: bool,
    #[structopt(short, long, help="keep (don't delete) input files")]
    keep: bool,
    #[structopt(short="l", long, help="list compressed file contents")]
    list: bool,
    #[structopt(short="L", long, help="display software license")]
    license: bool,
    #[structopt(short="n", long, help="don't save or restore original name and timestamp")]
    no_name: bool,
    #[structopt(short="N", long, help="save or restore original name and timestamp")]
    name: bool,
    #[structopt(short, long, help="suppress all warnings", conflicts_with="verbose")]
    quiet: bool,
    #[structopt(long, help="synchronous output (safer if system crashes, but slower)")]
    synchronous: bool,
    #[structopt(short, long, help="operate recursively on directories")]
    recursive: bool,
    #[structopt(short="S", long, help="use suffix SUF on compressed files", default_value=".gz")]
    suffix: String,
    #[structopt(short, long, help="test compressed file integrity")]
    test: bool,
    #[structopt(short="v", long, help="verbose mode", default_value="0")]
    verbose: u8,
    #[structopt(short="1", long, help="compress faster, but worse", conflicts_with="best")]
    fast: bool,
    #[structopt(short="9", long, help="compress better, but slower")]
    best: bool,
    #[structopt(long, help="specify compression level 1-9 (9: best, slow; 1: worst, fast)", default_value="6", parse(from_str="parse_level"))]
    level: i8,
    #[structopt(long, help="make rsync-friendly archive")]
    rsyncable: bool,
    #[structopt(short="j", long, help="compress in parallel with THREADS number of threads", default_value="1")]
    parallel: u8,
    no_time: bool,
    /// Files to process
    #[structopt(name = "FILE", parse(from_os_str), last = true)]
    files: Vec<PathBuf>
}

// TODO: make no_name and name exclusive arguments (e.g. cannot be passed together)
impl Opt {
    /// will error on being passed a suffix via --suffix greater than 30 characters
    /// checks to ensure that all command line arguments are consistent (e.g. quiet and verbose
    /// are not present at the same time). The following rules apply:
    ///     if --quiet is passed, --verbose is coerced to false/0
    ///     if --list is passed, we are getting statistics on compressed files. Therefore we are
    ///         decompressing.
    ///     if we are not restoring the name, we must also not be restoring the time
    ///     if we are testing a compressed file, we decompress, and we output to stdout
    ///     --ascii should only be present on windows systems
    fn new () -> Self {
        let mut opt = Opt::from_args();
        if opt.quiet {
            opt.verbose = 0;
        }
        if opt.list {
            opt.decompress = true;
            opt.stdout = true;
        }
        if opt.no_name {
            opt.no_time = true;
            opt.name = false;
        }
        if opt.name {
            opt.no_time = false;
            opt.no_name = false;
        }
        if opt.test {
            opt.decompress = true;
            opt.stdout = true;
        }
        if ! cfg!(target_os = "windows") {
            if opt.ascii && !opt.quiet {
                eprintln!("{}: option --ascii ignored on this system", constants::PROGRAM_NAME);
            }
            opt.ascii = false;
        }
        if opt.fast {
            opt.level = 1;
        }
        if opt.best {
            opt.level = 9;
        }
        // Default situation where both not given
        if !opt.name && !opt.no_name {
            opt.no_name = opt.decompress;
            opt.no_time = opt.decompress;
        }
        // Default to stdin
        if opt.files.is_empty() {
            opt.files = vec!(PathBuf::from("-"));
        }
        match check_if_suffix_too_long(&opt.suffix) {
            Some(_) => {
                eprintln!("{}: invalid suffix '{}'", constants::PROGRAM_NAME, opt.suffix);
                exit(constants::ERROR);
            },
            None => ()
        }
        opt.suffix = String::from(util::strip_leading_dot(opt.suffix.as_str()));
        opt
    }
}

fn parse_level(levelstr: &str) -> i8 {
    let mut level = match i8::from_str_radix(levelstr, 10) {
        Err(_e) => constants::DEFAULT_LEVEL,
        Ok(u) => u
    };
    if level > 9 {
        level = 9
    }
    else if level < 1 {
        level = 1
    }
    level
}

fn check_if_suffix_too_long (s: &String) -> Option<String> {
    if s.len() > constants::MAX_SUFFIX {
        Some(s);
    }
    None
}

fn print_license () {
    println!("Copyright (C) 2019 Free Software Foundation, Inc. Copyright (C) 1993 Jean-loup Gailly.\n\
     This is free software.  You may redistribute copies of it under the terms of\n\
     the GNU General Public License <https://www.gnu.org/licenses/gpl.html>.\n\
     There is NO WARRANTY, to the extent permitted by law.")
}

fn main() {
    let mut opt = Opt::new();
    if opt.license {
        print_license ();
    }
    let files = opt.files.clone();
    if opt.list {
        match list::do_list(files, &opt) {
            Ok(_) => return,
            Err(_) => exit(constants::ERROR)
        }
    }
    else{
        if files.is_empty (){
            treat::stdin (&mut opt);
        }
        treat::files(files, &mut opt);
    }
}
