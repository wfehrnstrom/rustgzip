use crate::{Opt, constants};
use std::convert::TryInto;
use std::io::{Error, ErrorKind};

pub trait List {
    const HEADER_SIZE: f64;

    fn list(&self, opt: &Opt) -> (f64, f64);

    fn month(m: u32) -> &'static str {
        match m {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???"
        }
    }

    fn calculate_ratio (compressed: f64, uncompressed: f64) -> f64 {
        let bytes_lost: f64 = uncompressed - (compressed - Self::HEADER_SIZE);
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
                    return Ok(Self::HEADER_SIZE)
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
}
