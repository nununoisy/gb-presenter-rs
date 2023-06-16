use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use sameboy::{Gameboy, JoypadButton};
use crate::renderer::SongPosition;

pub fn lsdj_select_track(gb: &mut Gameboy, track_index: u8) {
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

pub fn lsdj_get_song_position(gb: &mut Gameboy) -> Option<SongPosition> {
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

pub fn lsdj_sav_get_track_titles<P: AsRef<Path>>(sav_path: P) -> io::Result<Vec<String>> {
    let mut sav = BufReader::new(File::open(sav_path)?);
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

    Ok(titles)
}
