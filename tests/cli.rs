use std::convert::TryInto;
use std::io::Read;
use std::io::Write;
use std::process::*;  // Run programs
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::fs::{File, create_dir, remove_file};
use remove_dir_all::*;
use std::path::Path;

// THESE TESTS ARE ONLY GUARANTEED TO WORK ON UNIX. THEY HAVE NOT BEEN PORTED TO WINDOWS.

#[test]
fn file_doesnt_exist () -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::main_binary()?;
    cmd.arg("-d")
        .arg("--")
        .arg("testfiledoesntexist.txt.gz");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));
    let mut cmd = Command::main_binary()?;
    cmd.arg("--")
       .arg("testfiledoesntexist.txt");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));
    Ok(())
}

#[test]
fn gzip_compat () -> Result<(), Box<dyn std::error::Error>> {
    let msg = b"testing....testing....testing...This is major tom.";
    let mut file = File::create("test.txt")?;
    file.write_all(msg)?;
    let mut rstzip = Command::main_binary()?;
    rstzip.arg("--")
        .arg("test.txt");
    rstzip.assert()
        .success();
    let mut mv = Command::new("mv");
    mv.arg("test.txt.gz")
        .arg("test2.txt.gz");
    mv.assert()
        .success();
    let mut file2 = File::create("test.txt")?;
    file2.write_all(msg)?;
    let mut gzip = Command::new("./ref-gzip");
    gzip.arg("test.txt");
    gzip.assert()
        .success();
    let mut diff = Command::new("diff");
    diff.arg("-u")
        .arg("test.txt.gz")
        .arg("test2.txt.gz");
    diff.assert()
        .success();
    Ok(())
}

// fn decompresses_gz () -> Result<(), Box<dyn std::error::Error>> {
//
// }
//
#[cfg(not(windows))]
#[test]
fn ascii_unsupported () -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::main_binary()?;
    cmd.arg("--ascii")
        .arg("--")
        .arg("");
    cmd.assert()
        .stderr(predicate::str::contains("option --ascii ignored on this system"));
    Ok(())
}

#[test]
fn dir () -> Result<(), Box<dyn std::error::Error>> {
    create_dir("temp")?;
    let mut file1 = File::create("temp/file1")?;
    let mut file2 = File::create("temp/file2")?;
    file1.write_all(b"Something great")?;
    file2.write_all(b"Something greater")?;
    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["--", "temp"]);
    // assert that rust returns warning code
    rstzip.assert().code(2);
    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-r", "--", "temp"]);
    rstzip.assert().success();

    let mut find = Command::new("find");
    find.args(&["temp", "(", "-regex", ".*gz$", "-print", ")"]);
    let output = find.output()?;
    let out = output.stdout;
    let mut wc = Command::new("wc").arg("-l").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let stdin = wc.stdin.as_mut().unwrap();
    stdin.write_all(std::str::from_utf8(out.as_slice()).unwrap().as_bytes())?;
    let output = wc.wait_with_output()?;
    output.assert().stdout(predicate::str::contains("2"));

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-dr", "--", "temp"]);
    rstzip.assert().success();

    let mut find = Command::new("find");
    find.args(&["temp", "(", "-regex", ".*gz$", "-print", ")"]);
    let output = find.output()?;
    let out = output.stdout;
    let mut wc = Command::new("wc").arg("-l").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let stdin = wc.stdin.as_mut().unwrap();
    stdin.write_all(std::str::from_utf8(out.as_slice()).unwrap().as_bytes())?;
    let output = wc.wait_with_output()?;
    output.assert().stdout(predicate::str::contains("0"));

    remove_dir_all("temp")?;
    Ok(())
}

#[test]
fn name_and_no_name () -> Result<(), Box<dyn std::error::Error>> {
    let mut file1 = File::create("file1")?;
    file1.write_all(b"Something great")?;
    // check that upon being instructed to store no name, that rstzip does so
    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-n", "--", "file1"]);
    rstzip.assert().success();

    let mut mv = Command::new("mv");
    mv.args(&["file1.gz", "file2.gz"]);
    mv.assert().success();

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-dN", "--", "file2.gz"]).assert().success();

    let mut find = Command::new("find");
    find.args(&[".", "(", "-regex", "\\./file2", "-print", ")"]);
    let output = find.output()?;
    let out = output.stdout;
    let err = output.stderr;
    assert_eq!(std::str::from_utf8(err.as_slice()).unwrap(), "");
    let mut wc = Command::new("wc").arg("-l").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let stdin = wc.stdin.as_mut().unwrap();
    stdin.write_all(std::str::from_utf8(out.as_slice()).unwrap().as_bytes())?;
    let output = wc.wait_with_output()?;
    output.assert().stdout(predicate::str::contains("1"));

    // ensure that the default mode is to store the filename in the compressed file, and given -N,
    // this doesn't change the behavior either
    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["--", "file2"]);
    rstzip.assert().success();

    let mut mv = Command::new("mv");
    mv.args(&["file2.gz", "file3.gz"]);
    mv.assert().success();

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-dN", "--", "file3.gz"]).assert().success();

    let mut find = Command::new("find");
    find.args(&[".", "(", "-regex", "\\./file2", "-print", ")"]);

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-N", "--", "file2"]);
    rstzip.assert().success();

    let mut mv = Command::new("mv");
    mv.args(&["file2.gz", "file3.gz"]);
    mv.assert().success();

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-dN", "--", "file3.gz"]).assert().success();

    let mut find = Command::new("find");
    find.args(&[".", "(", "-regex", "\\./file2", "-print", ")"]);

    let output = find.output()?;
    let out = output.stdout;
    let err = output.stderr;
    assert_eq!(std::str::from_utf8(err.as_slice()).unwrap(), "");
    let mut wc = Command::new("wc").arg("-l").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let stdin = wc.stdin.as_mut().unwrap();
    stdin.write_all(std::str::from_utf8(out.as_slice()).unwrap().as_bytes())?;
    let output = wc.wait_with_output()?;
    output.assert().stdout(predicate::str::contains("1"));

    remove_file("file2")?;
    Ok(())
}

#[test]
fn keep() -> Result<(), Box<dyn std::error::Error>> {
    File::create("keep")?;

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-k", "--", "keep"]);
    rstzip.assert().success();

    assert!(Path::new("./keep").exists());

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-dk", "--", "keep.gz"]);
    rstzip.assert().success();

    assert!(Path::new("./keep.gz").exists());

    remove_file("keep.gz")?;
    remove_file("keep")?;
    Ok(())
}

#[test]
fn list() -> Result<(), Box<dyn std::error::Error>> {
    let f2_orig_name = "list2";
    let mut f1 = File::create("list1")?;
    let mut f2 = File::create(f2_orig_name)?;
    let gettysburg_addr = b"Four score and seven years ago our fathers brought forth on this continent, a new nation, conceived in Liberty, and dedicated to the proposition that all men are created equal.\
        Now we are engaged in a great civil war, testing whether that nation, or any nation so conceived and so dedicated, can long endure. We are met on a great battle-field of that war. We have come to dedicate a portion of that field, as a final resting place for those who here gave their lives that that nation might live. It is altogether fitting and proper that we should do this. \
        But, in a larger sense, we can not dedicate -- we can not consecrate -- we can not hallow -- this ground. The brave men, living and dead, who struggled here, have consecrated it, far above our poor power to add or detract. The world will little note, nor long remember what we say here, but it can never forget what they did here. It is for us the living, rather, to be dedicated here to the unfinished work which they who fought here have thus far so nobly advanced. It is rather
        for us to be here dedicated to the great task remaining before us -- that from these honored dead we take increased devotion to that cause for which they gave the last full measure of devotion -- that we here highly resolve that these dead shall not have died in vain -- that this nation, under God, shall have a new birth of freedom -- and that government of the people, by the people, for the people, shall not perish from the earth.";
    let f1_uncompr_len = 1480;
    let f1_uncompr_len_str = format!("{}", f1_uncompr_len);
    f1.write_all(gettysburg_addr)?;
    let gw = b"We are all Democrats, we are all Republicans";
    let f2_uncompr_len = 44;
    let f2_uncompr_len_str = format!("{}", f2_uncompr_len);
    let total_uncompr_len = format!("{}", f1_uncompr_len + f2_uncompr_len);
    f2.write_all(gw)?;
    f1.flush()?;
    f2.flush()?;

    // modify mtime to some known, in bounds value, and check for it later under --verbose 1
    let mut touch = Command::new("touch");
    touch.args(&["-mt", "0711171533", "./list1"]);
    touch.assert().success();

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["--", "list1", f2_orig_name]);
    rstzip.assert().success();

    let f1_compr_len = File::open("list1.gz")?.metadata()?.len();
    let f2_compr_len = File::open("list2.gz")?.metadata()?.len();
    let total_compr_len = format!("{}", f1_compr_len + f2_compr_len);
    let header_size = 18.0;

    let f1_compr_ratio = format!("{:.1}", ((to_float(f1_uncompr_len) - (to_float(f1_compr_len)-header_size))*100.0)/to_float(f1_uncompr_len));

    let f1_compr_len_str = format!("{}", f1_compr_len);
    let f2_compr_len_str = format!("{}", f2_compr_len);

    let mut mv = Command::new("mv");
    mv.args(&["list2.gz", "othername.gz"]);
    mv.assert().success();

    let mut rstzip = Command::main_binary()?;
    // this means we should use original names
    rstzip.args(&["-lN", "--", "list1.gz", "othername.gz"]);
    let out = rstzip.output()?;
    let list_output = std::str::from_utf8(out.stdout.as_slice())?;
    assert!(list_output.contains(f1_uncompr_len_str.as_str()));
    assert!(list_output.contains(f2_uncompr_len_str.as_str()));
    assert!(list_output.contains(f1_compr_len_str.as_str()));
    assert!(list_output.contains(f2_compr_len_str.as_str()));
    assert!(list_output.contains(f2_orig_name));

    let mut rstzip = Command::main_binary()?;
    // this means we should use generated names -- i.e. strip off the .gz
    rstzip.args(&["-l", "--", "list1.gz", "othername.gz"]);
    let out = rstzip.output()?;
    let list_output = std::str::from_utf8(out.stdout.as_slice())?;

    assert!(list_output.contains("othername"));
    eprintln!("compr ratio: {}", f1_compr_ratio.as_str());
    assert!(list_output.contains(f1_compr_ratio.as_str()));

    let mut rstzip = Command::main_binary()?;
    // now we check for the additional fields specified in verbose mode

    rstzip.args(&["-l", "--verbose", "1", "--", "list1.gz", "othername.gz"]);
    let out = rstzip.output()?;
    let stdout_str = std::str::from_utf8(out.stdout.as_slice())?;

    assert!(stdout_str.contains("defla"));
    assert!(stdout_str.contains("Nov 17"));
    assert!(stdout_str.contains("15:33"));
    assert!(stdout_str.contains("(totals)"));
    assert!(stdout_str.contains(total_uncompr_len.as_str()));
    assert!(stdout_str.contains(total_compr_len.as_str()));
    // TODO: check for correct crc.

    remove_file("list1.gz")?;
    remove_file("othername.gz")?;

    let mut rstzip = Command::main_binary()?;
    // now check for combination of verbose and quiet modes
    // actually, this can't happen, because structopt was contrived to not allow it. Therefore,
    // this is not up to hoped for gzip-compatibility. TODO: make compatible.
    rstzip.args(&["-lq", "--verbose", "1", "--", "list1.gz", "othername.gz"]);
    let out = rstzip.output()?;
    let stdout_str = std::str::from_utf8(out.stdout.as_slice())?;


    assert!(!stdout_str.contains("defla"));
    assert!(!stdout_str.contains("(totals)"));
    assert!(!stdout_str.contains("method"));
    assert!(!stdout_str.contains("compressed"));

    Ok(())
}

// this function is unsafe and prone to failure because it downsizes values. Use with caution.
fn to_float (val: u64) -> f64 {
    let val_u32: u32 = val.try_into().unwrap();
    val_u32.try_into().unwrap()
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
    let mut f1 = match File::create("test1") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("couldn't create file with error: {}", e);
            return Err(Box::new(e));
        }
    };
    let msg = b"That's one small step for man, one giant leap for mankind";
    match f1.write_all(msg) {
        Ok(..) => {},
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(e));
        }
    }

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["--", "test1"]);
    rstzip.assert().success();

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-t", "--verbose", "1", "--", "test1.gz"]);
    rstzip.assert().success().stdout(predicate::str::contains("OK"));

    let mut bytes_read: Vec<u8> = Vec::new();
    let mut readfd = File::open("test1.gz")?;
    match readfd.read_to_end(&mut bytes_read){
        Ok(..) => {},
        Err(e) => {
            eprintln!("error while reading file: {}", e);
            return Err(Box::new(e));
        }
    }
    bytes_read.insert(5, 54);
    bytes_read.insert(3, 20);
    bytes_read.insert(8, 45);
    bytes_read.insert(16, 39);
    bytes_read.insert(35, 128);

    remove_file("test1.gz")?;
    let mut f2 = File::create("test2.gz")?;
    f2.write_all(bytes_read.as_mut_slice())?;

    let mut rstzip = Command::main_binary()?;
    rstzip.args(&["-t", "--verbose", "1", "--", "test2.gz"]);
    rstzip.assert().failure();

    remove_file("test2.gz")?;

    Ok(())
}
