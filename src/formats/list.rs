use std::path::PathBuf;
use crate::{Opt, constants, util};
use std::convert::TryInto;
use std::io::{Error, ErrorKind};
use chrono::{DateTime, Datelike, Timelike};
use chrono::offset::Local;
use std::str::FromStr;

pub trait List {

    fn list(&self, opt: &Opt) -> (f64, f64);

    fn header_size(&self) -> f64;

    fn month(m: u32) -> &'static str where Self: Sized {
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

    fn calculate_ratio (compressed: f64, uncompressed: Option<f64>, header_size: f64) -> f64 where Self: Sized  {
        let uncompressed = match uncompressed {
            Some(n) => n,
            None => return 0.0
        };
        let bytes_lost: f64 = uncompressed - (compressed - header_size);
        let ratio: f64 = (bytes_lost/uncompressed).try_into().unwrap();
        ratio*100.0
    }

    // check that the number of bytes is not negative. If it is and we aren't forcing,
    // return an error after emitting error msg
    fn bytes_bound_check (num_bytes: f64, opt: &Opt, compressed: bool, header_size: f64) -> std::io::Result<f64> where Self: Sized {
        if num_bytes < 18.0 {
            if opt.verbose > 1 {
                eprintln!("{}: internal error: do_list: the number of bytes in a file\
                    cannot be less than the header size", constants::PROGRAM_NAME);
            }
            if opt.force {
                if compressed {
                    return Ok(header_size)
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

    fn datestring (dt: &DateTime<Local>) -> String where Self: Sized {
        return Self::datestring_raw (dt.month().try_into().unwrap(), dt.day().try_into().unwrap());
    }

    fn datestring_raw (month: u8, day: u8) -> String where Self: Sized {
        return format!("{} {}", Self::month(month.into()), day);
    }

    fn timestring (dt: &DateTime<Local>) -> String where Self: Sized {
        return Self::timestring_raw (dt.hour().try_into().unwrap(), dt.minute().try_into().unwrap());
    }

    fn timestring_raw(hour: u8, minute: u8) -> String where Self:Sized {
        return format!("{}:{:02}", hour, minute);
    }

    fn get_filename_str (stored_filename: &Option<String>, path: &Option<PathBuf>, opt: &Opt)
        -> String where Self: Sized
        {
        if opt.name {
            match stored_filename {
                Some(s) => String::from(s),
                None => String::from("????")
            }
        }
        else {
             if let Some(p) = path {
                 let filename = match util::make_ofname(&p, opt) {
                        Ok(boxed_path_buf) => boxed_path_buf,
                        Err(_) => Box::new(PathBuf::from_str("????").unwrap())
                 };
                 String::from((*filename).to_str().unwrap())
             }
             else{
                 String::from("????")
             }
        }
    }
}
