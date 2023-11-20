use anyhow::{Result, bail};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub fn get_track_titles_from_save<P: AsRef<Path>>(sav_path: P) -> Result<Vec<String>> {
    let mut sav = BufReader::new(File::open(sav_path)?);

    sav.seek(SeekFrom::Start(0x813E))?;
    let mut jk = [0u8; 2];
    sav.read_exact(&mut jk)?;
    if &jk != b"jk" {
        bail!("Invalid LSDj save file!");
    }

    sav.seek(SeekFrom::Start(0x8000))?;

    let mut titles = [0u8; 0x100];
    sav.read_exact(&mut titles)?;

    let mut versions = [0u8; 0x20];
    sav.read_exact(&mut versions)?;

    let mut result: Vec<String> = Vec::new();

    for (raw_title, version) in std::iter::zip(titles.chunks_exact(8), versions) {
        let title: Vec<u8> = raw_title
            .iter()
            .take_while(|&&b| b != 0)
            .cloned()
            .collect();

        if title.is_empty() {
            break;
        }

        result.push(format!("{}.{:X}", String::from_utf8(title).unwrap(), version));
    }

    if result.is_empty() {
        bail!("LSDj save is empty!");
    }

    Ok(result)
}