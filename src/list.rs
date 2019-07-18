use crate::util::WrappedFile;
use crate::Opt;
use std::path::PathBuf;
use crate::util;
use std::convert::TryFrom;
use crate::formats::gz::GzFile;
use crate::formats::list::List;


pub fn do_list (files: Vec<PathBuf>, opt: &Opt) -> std::io::Result<()> {
    if files.len() > 0 {
        if opt.verbose > 0 {
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
    for filepath in files {
        let file = util::file_open(&filepath)?;
        let wfile = WrappedFile { path: filepath.as_path(), file: &file};
        let gz_file = match GzFile::try_from(wfile) {
            Ok(f) => f,
            Err(_) => continue
        };
        let (compressed_bytes, uncompressed_bytes) = gz_file.list(opt);
        total_compressed_bytes += compressed_bytes;
        total_uncompressed_bytes += uncompressed_bytes;
    }
    if num_files > 1 {
        let total_ratio = GzFile::calculate_ratio(total_compressed_bytes, total_uncompressed_bytes);
        if opt.verbose > 0 {
            print!("{:>36}", " ");
        }
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t",
            total_compressed_bytes, total_uncompressed_bytes, total_ratio, "(totals)");
    }
    Ok(())
}
