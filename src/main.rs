use std::{
    env::current_dir, error::Error, fs::{create_dir_all, File}, io::{Read, Write}, os::unix::fs::FileExt, path::{Path, PathBuf}, usize
};
use compress::zlib;

static USAGE: &str = r#"
Usage:
extracthvp <in> [out]"#;

static TAG: &[u8] = "HV PackFile".as_bytes();

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("{}", USAGE);
        return Ok(());
    }
    let in_file: &str = &args[1];
    let out_dir = if args.len() > 2 { PathBuf::from(&args[2]) } else { current_dir().unwrap() };
    let out_dir = Path::new(&out_dir);
    if !out_dir.exists() {
        println!("ERROR: Output directory {} does not exist!", out_dir.to_str().unwrap_or("unknown"));
        return Ok(());
    }

    let mut hvp = File::open(in_file)?;
    let mut buf = [0; 11];
    let _ = hvp.read_exact(&mut buf);
    if buf != TAG {
        println!("ERROR: {} is not a valid HV PackFile", in_file);
        return Ok(());
    }
    skip_bytes(&mut hvp, 5);
    let n = read_integer(&mut hvp);
    skip_bytes(&mut hvp, 20);
    for _ in 0..n {
        read_next(&mut hvp, out_dir);
    }

    Ok(())
}

fn read_next(file: &mut File, path: &Path) {
    skip_bytes(file, 4);
    let file_type = read_one(file);
    if file_type != 0 {
        read_file(file, path);
    } else {
        read_directory(file, path);
    }
}

// 4 - ???
// 4 - no of files
// 4 - length of the name
// x - the name
fn read_directory(file: &mut File, path: &Path) {
    skip_bytes(file, 4);
    let no_of_files = read_integer(file);
    let name_length = read_integer(file);
    let name = read_bytes(file, name_length.try_into().unwrap());
    let name = String::from_utf8(name).unwrap();
    let path = path.join(name);
    create_dir(&path);
    for _ in 0..no_of_files {
        read_next(file, &path);
    }
}

// 4 - 1 -> is compressed
// 4 - the size of the compressed data
// 4 - the size of the uncompressed data
// 4 - ???
// 4 - the offset from the start of the file where the data resides
// 4 - length of the name
// x - the name
//
fn read_file(file: &mut File, path: &Path) {
    let is_compressed = read_integer(file);
    let comp_size = read_integer(file);
    let size = read_integer(file);
    skip_bytes(file, 4);
    let offset = read_integer(file);
    let name_length = read_integer(file);
    let name = read_bytes(file, name_length.try_into().unwrap());
    let name = String::from_utf8(name).unwrap();
    let data = if is_compressed != 0 {
        read_compressed(file, offset, comp_size.try_into().unwrap(), size.try_into().unwrap())
    } else {
        read_uncompressed(file, offset, size.try_into().unwrap())
    };
    let path = path.join(name);
    let mut out_file = create_file(&path);
    _ = out_file.write_all(&data);
}

fn read_compressed(file: &mut File, offset: u32, comp_size: usize, size: usize) -> Vec<u8> {
    let compressed = read_uncompressed(file, offset, comp_size);
    let mut decompressed = vec![0; size];
    _ = zlib::Decoder::new(compressed.as_slice()).read_to_end(&mut decompressed);
    decompressed
}

fn read_uncompressed(file: &mut File, offset: u32, size: usize) -> Vec<u8> {
    let mut buf = vec![0; size];
    _ = file.read_exact_at(&mut buf, offset.into());
    buf
}

fn create_dir(path: &Path) {
    println!("Creating dir {}", path.display());
    _ = create_dir_all(path.to_str().unwrap());
}

fn create_file(path: &Path) -> File {
    println!("Creating file {}", path.to_str().unwrap_or(""));
    File::create(path).unwrap()
}

fn skip_bytes(file: &mut File, bytes: usize) {
    _ = read_bytes(file, bytes);
}

fn read_integer(file: &mut File) -> u32 {
    u32::from_be_bytes(read_four(file))
}

fn read_one(file: &mut File) -> i32 {
    let val = read_bytes(file, 1);
    val[0].into()
}

fn read_four(file: &mut File) -> [u8; 4] {
    let mut buf = [0; 4];
    _ = file.read_exact(&mut buf);
    buf
}

fn read_bytes(file: &mut File, bytes: usize) -> Vec<u8> {
    let mut buf = vec![0; bytes];
    _ = file.read_exact(&mut buf);
    buf
}
