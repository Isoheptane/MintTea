use std::path::Path;
use std::io::Write;

use tempfile::NamedTempFile;

pub fn create_tempfile<P: AsRef<Path>>(name: &str, dir: P, random: usize) -> Result<NamedTempFile, std::io::Error> {
    tempfile::Builder::new()
        .rand_bytes(random)
        .suffix(name)
        .tempfile_in(dir)
}

pub fn save_to_tempfile<P: AsRef<Path>>(name: &str, dir: P, random: usize, content: Vec<u8>) -> Result<NamedTempFile, std::io::Error> {
    let mut tempfile = create_tempfile(name, dir, random)?;
    tempfile.write_all(&content)?;
    Ok(tempfile)
}