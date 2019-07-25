use crate::util::WrappedFile;
use crate::{Opt, constants};
use std::path::PathBuf;
use crate::util;
use crate::formats::parse_list;
use crate::formats::gz::GzFile;
use crate::formats::list::List;

pub fn do_list (files: Vec<PathBuf>, opt: &Opt) -> Result<(), i8> {
    if files.len() > 0 {
        if opt.verbose > 0 && !opt.quiet {
            print!("{:<8}{:<12}{:<8}{:<8}", "method", "crc", "date", "time");
        }
        if !opt.quiet {
            println!("{:<8}\t{:<8}\t{:>8}\t{:<8}", "compressed", "uncompressed", "ratio", "uncompressed_name");
        }
    }
    let mut total_compressed_bytes = 0.0;
    let mut total_uncompressed_bytes = 0.0;
    // I know that reading the file is potentially slow
    // TODO: Look for a faster solution. The use of libc::lseek is hindered by abstracting away ifd to
    // this point, so reintroducing it here would be an incongruity.
    let num_files = files.len();
    let mut header_size: f64 = 18.0;
    let mut i = 1.0;
    for filepath in files {
        let file = match util::file_open(&filepath) {
            Ok(f) => f,
            Err(_) => return Err(constants::ERROR)
        };
        let wfile = WrappedFile { path: filepath.as_path(), file: &file};
        let compr_file: Box<dyn List> = parse_list(wfile);
        header_size = (header_size + compr_file.header_size())/i;
        i += 1.0;
        let (compressed_bytes, uncompressed_bytes) = compr_file.list(opt);
        total_compressed_bytes += compressed_bytes;
        total_uncompressed_bytes += uncompressed_bytes;
    }
    if num_files > 1 {
        let total_ratio = GzFile::calculate_ratio(total_compressed_bytes, Some(total_uncompressed_bytes), header_size);
        if opt.verbose > 0 {
            print!("{:>36}", " ");
        }
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t",
            total_compressed_bytes, total_uncompressed_bytes, total_ratio, "(totals)");
    }
    Ok(())
}
