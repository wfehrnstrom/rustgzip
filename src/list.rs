use crate::Opt;
use std::path::PathBuf;
use crate::util;
use crate::constants;
use std::io::{Error, ErrorKind, Read};
use std::convert::TryInto;

const GZ_HEADER_BYTES: f64 = 18.0;

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
        // header_bytes should really be not statically set but discovered, but for this initial
        // stage, we only do deflate files, so we can just set it and forget
        // TODO: add support for checking files compressed in a way other than deflate.
        // Includes trailer
        let mut file = util::file_open (&filepath)?;
        let mut bytes: Vec<u8> = Vec::new();
        file.read_to_end(&mut bytes)?;
        let compressed_size: u32 = bytes.len().try_into().unwrap();
        let compressed_size: f64 = match bytes_bound_check(compressed_size.try_into().unwrap(), opt, true) {
            Ok(bytes) => bytes,
            Err(e) => return Err(e)
        };
        total_compressed_bytes += compressed_size;

        let mut trailer: Vec<u8> = bytes.split_off (bytes.len() - 8);
        if trailer.len () < 8 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "compressed input file less than 8 bytes: illegal"));
        }
        let crc = util::shift_left(4, trailer.as_slice());
        let isize_vec = trailer.split_off(4);
        let uncompressed_size: f64 = match bytes_bound_check(util::shift_left(4, isize_vec.as_slice()).try_into().unwrap(), opt, false) {
            Ok(bytes) => bytes,
            Err(e) => return Err(e)
        };
        total_uncompressed_bytes += uncompressed_size;
        let percent_ratio = calculate_ratio(compressed_size, uncompressed_size, GZ_HEADER_BYTES);
        let uncompressed_filename = match util::make_ofname(&filepath, opt) {
            Ok(boxed_path_buf) => boxed_path_buf,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Error constructing
                new output file name"))
        };
        let uncompressed_filename: &str = (*uncompressed_filename).to_str().unwrap();
        if opt.verbose > 0 {
            print!("{:<8}{:<12x}{:<8}{:<8}", "defla", crc, "Jan 10", "15:43")
        }
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t", compressed_size,
            uncompressed_size, percent_ratio, uncompressed_filename);
    }
    if num_files > 1 {
        let total_ratio = calculate_ratio(total_compressed_bytes, total_uncompressed_bytes, GZ_HEADER_BYTES);
        if opt.verbose > 0 {
            print!("{:>36}", " ");
        }
        println!("{:<8}\t{:<8}\t{:>8.1}%\t{:<8}\t",
            total_compressed_bytes, total_uncompressed_bytes, total_ratio, "(totals)");
    }
    Ok(())
}

// all arguments are f64 so that we can display precise compression ratio
fn calculate_ratio (compressed: f64, uncompressed: f64, header_bytes: f64) -> f64 {
    let bytes_lost: f64 = uncompressed - (compressed - header_bytes);
    let ratio: f64 = (bytes_lost/uncompressed).try_into().unwrap();
    ratio*100.0
}

// check that the number of bytes is not negative. If it is and we aren't forcing,
// return an error after emitting error msg
fn bytes_bound_check (num_bytes: f64, opt: &Opt, compressed: bool) -> std::io::Result<f64> {
    if num_bytes < 18.0 {
        if opt.verbose > 1 {
            eprintln!("{}: internal error: do_list: the number of bytes in a file\
                cannot be less than the header size", constants::PROGRAM_NAME);
        }
        if opt.force {
            if compressed {
                return Ok(GZ_HEADER_BYTES)
            }
            else{
                if num_bytes < 0.0 {
                    return Ok(0.0)
                }
                return Ok(num_bytes)
            }
        }
        return Err(Error::new(ErrorKind::InvalidData, "internal error: do_list: the number of bytes\
        in a file cannot be less than the header size"))
    }
    return Ok(num_bytes)
}
