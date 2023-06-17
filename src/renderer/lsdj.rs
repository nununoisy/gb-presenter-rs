use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use sameboy::{Gameboy, JoypadButton};
use crate::renderer::SongPosition;

pub fn select_track_joypad_macro(gb: &mut Gameboy, track_index: u8) {
    // Open the project menu
    gb.joypad_macro_frame(&[JoypadButton::Select, JoypadButton::Up]);
    gb.joypad_macro_frame(&[JoypadButton::Select, JoypadButton::Up]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    // Scroll to the bottom option
    for _ in 0..32 {
        gb.joypad_macro_frame(&[JoypadButton::Down]);
        gb.joypad_macro_frame(&[JoypadButton::Down]);
        gb.joypad_macro_frame(&[]);
        gb.joypad_macro_frame(&[]);
    }
    // Select Load/Save
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    // Select Load
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    // Scroll to the topmost song
    for _ in 0..32 {
        gb.joypad_macro_frame(&[JoypadButton::Up]);
        gb.joypad_macro_frame(&[JoypadButton::Up]);
        gb.joypad_macro_frame(&[]);
        gb.joypad_macro_frame(&[]);
    }
    // Scroll down to the desired song and select it
    for _ in 0..track_index {
        gb.joypad_macro_frame(&[JoypadButton::Down]);
        gb.joypad_macro_frame(&[JoypadButton::Down]);
        gb.joypad_macro_frame(&[JoypadButton::Down]);
        gb.joypad_macro_frame(&[]);
        gb.joypad_macro_frame(&[]);
    }
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    // Dismiss the "save changes?" dialog if it appears
    gb.joypad_macro_frame(&[JoypadButton::Up]);
    gb.joypad_macro_frame(&[JoypadButton::Up]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[JoypadButton::Left]);
    gb.joypad_macro_frame(&[JoypadButton::Left]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[]);
    gb.joypad_macro_frame(&[JoypadButton::A]);
    gb.joypad_macro_frame(&[JoypadButton::A]);
    // Wait for the song to load
    for _ in 0..256 {
        gb.joypad_macro_frame(&[]);
    }
}

const LSDJ_ROW_BASE_ADDR: u16 = 0xC200;

pub fn get_song_position(gb: &mut Gameboy) -> Option<SongPosition> {
    // TODO: detect HFF condition
    let position = (0..4)
        .map(|i| gb.read_memory(LSDJ_ROW_BASE_ADDR + i))
        .filter(|&r| r <= 0x7F)
        .max();

    match position {
        Some(row) => Some(SongPosition { row, end: false }),
        None => None
    }
}

pub fn get_lsdj_version<P: AsRef<Path>>(rom_path: P) -> io::Result<Option<String>> {
    let mut rom = BufReader::new(File::open(rom_path)?);
    rom.seek(SeekFrom::Start(0x134))?;

    let mut title_bytes = vec![0u8; 11];
    rom.read_exact(&mut title_bytes)?;

    if let Some(terminator) = title_bytes.iter().rposition(|&b| b != 0) {
        title_bytes.truncate(terminator + 1);
    }
    let title = match String::from_utf8(title_bytes) {
        Ok(title) => title,
        Err(_) => return Ok(None)
    };

    if !title.starts_with("LSDj-v") {
        return Ok(None)
    }
    Ok(Some(title.replace("LSDj-v", "")))
}

pub fn get_track_titles_from_save<P: AsRef<Path>>(sav_path: P) -> io::Result<Option<Vec<String>>> {
    let mut sav = BufReader::new(File::open(sav_path)?);

    sav.seek(SeekFrom::Start(0x813E))?;
    let mut jk = [0u8; 2];
    sav.read_exact(&mut jk)?;
    if &jk != b"jk" {
        return Ok(None);
    }

    sav.seek(SeekFrom::Start(0x8000))?;
    let mut titles = [0u8; 0x100];
    sav.read_exact(&mut titles)?;

    let titles: Vec<String> = titles.chunks_exact(8)
        .filter(|c| c.iter().any(|&b| b != 0))
        .map(|c| {
            let c = c.iter().filter(|&&b| b != 0).cloned().collect();
            String::from_utf8(c).unwrap()
        })
        .collect();

    Ok(Some(titles))
}
