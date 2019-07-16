use crate::Opt;
use std::path::PathBuf;
use crate::util;
use std::io::{Error, ErrorKind, Read};
use std::convert::TryInto;

pub fn do_list (files: Vec<PathBuf>, opt: &Opt) -> std::io::Result<()> {
    if files.len() > 0 {
        // if opt.verbose > 0 {
        //     print!("{:<8}{:<8}{:<8}{:<8}", "method", "crc", "date", "time");
        // }
        if !opt.quiet {
            println!("{:<8}\t{:<8}\t{:>8}\t{:<8}", "compressed", "uncompressed", "ratio", "uncompressed_name");
        }
    }
    // I know that reading the file is potentially slow
    // TODO: Look for a faster solution. The use of libc::lseek is hindered by abstracting away ifd to
    // this point, so reintroducing it here would be an incongruity.
    for filepath in files {
        // header_bytes should really be not statically set but discovered, but for this initial
        // stage, we only do deflate files, so we can just set it and forget
        // TODO: add support for checking files compressed in a way other than deflate.
        // Includes trailer
        let header_bytes = 18.0;
        let mut file = util::file_open (&filepath)?;
        let mut bytes: Vec<u8> = Vec::new();
        file.read_to_end(&mut bytes)?;
        let compressed_size: u32 = bytes.len().try_into().unwrap();
        let mut compressed_size: f64 = compressed_size.try_into().unwrap();
        // To prevent ratio from potentially being greater than 1
        if compressed_size < 18.0 {
            compressed_size = 18.0;
        }
        let mut trailer: Vec<u8> = bytes.split_off (bytes.len() - 8);
        if trailer.len () < 8 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "compressed input file less than 8 bytes: illegal"));
        }
        let _crc = util::shift_left(4, trailer.as_slice());
        let isize_vec = trailer.split_off(4);
        let uncompressed_size: f64 = util::shift_left(4, isize_vec.as_slice()).try_into().unwrap();
        let bytes_lost: f64 = uncompressed_size - (compressed_size - header_bytes);
        let ratio: f64 = (bytes_lost/uncompressed_size).try_into().unwrap();
        let percent_ratio = ratio*100.0;
        let uncompressed_filename = match util::make_ofname(&filepath, opt) {
            Ok(boxed_path_buf) => boxed_path_buf,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Error constructing
                new output file name"))
        };
        let uncompressed_filename: &str = (*uncompressed_filename).to_str().unwrap();
        if opt.verbose > 0 {

        }
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", compressed_size,
            uncompressed_size, percent_ratio, uncompressed_filename);
    }
    Ok(())
}
