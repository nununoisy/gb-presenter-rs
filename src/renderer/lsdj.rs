use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use sameboy::{Gameboy, JoypadButton};
use crate::renderer::SongPosition;

pub fn select_track_joypad_macro(gb: &mut Gameboy, track_index: u8) {
    // Skip LittleFM screen if enabled
    gb.joypad_macro_press(&[JoypadButton::B], None);
    gb.joypad_macro_press(&[], None);
    // Open the project menu
    gb.joypad_macro_press(&[JoypadButton::Select], None);
    gb.joypad_macro_press(&[JoypadButton::Select, JoypadButton::Up], None);
    gb.joypad_macro_press(&[], None);
    // Scroll to the bottom option
    for _ in 0..16 {
        gb.joypad_macro_press(&[JoypadButton::Down], None);
        gb.joypad_macro_press(&[], None);
    }
    // Select Load/Save
    gb.joypad_macro_press(&[JoypadButton::A], None);
    gb.joypad_macro_press(&[], None);
    // Select Load
    gb.joypad_macro_press(&[JoypadButton::A], None);
    gb.joypad_macro_press(&[], None);
    // Scroll to the topmost song
    for _ in 0..32 {
        gb.joypad_macro_press(&[JoypadButton::Up], None);
        gb.joypad_macro_press(&[], None);
    }
    // Scroll down to the desired song
    for _ in 0..track_index {
        gb.joypad_macro_press(&[JoypadButton::Down], None);
        gb.joypad_macro_press(&[], None);
    }
    // Select the song
    gb.joypad_macro_press(&[JoypadButton::A], None);
    gb.joypad_macro_press(&[], None);
    // Dismiss the "save changes?" dialog if it appears
    gb.joypad_macro_press(&[JoypadButton::Left], None);
    gb.joypad_macro_press(&[], None);
    gb.joypad_macro_press(&[JoypadButton::A], None);
    // Wait for the song to load
    gb.joypad_macro_press(&[], Some(Duration::from_secs(5)));
}

pub fn get_song_position(gb: &mut Gameboy) -> Option<SongPosition> {
    let lsdj_version = get_running_lsdj_version(gb)?;
    let mut component_iter = lsdj_version.split(".");
    let major = i32::from_str(component_iter.next().unwrap_or_default()).unwrap_or(0);
    let minor = i32::from_str(component_iter.next().unwrap_or_default()).unwrap_or(0);
    let revision = i32::from_str(component_iter.next().unwrap_or_default()).unwrap_or(0);

    let base_addr = match (major, minor, revision) {
        (0 ..= 2, _, _) => return None,
        (3, 1, 9) => 0xC553,
        (3, 6, _) => 0xC544,
        (3, 7, _) => 0xC544,
        (3, 8, _) => 0xC599,
        (3, 9, _) => 0xC599,
        (3, _, _) => 0xC552,
        (4, 0, _) => 0xC351,
        (4, 1, _) => 0xC598,
        (4, 3, _) => 0xC499,
        (4, 4, _) => 0xC497,
        (4, 5, _) => 0xC492,
        (4, 6, _) => 0xC492,
        (4, 7, _) => 0xC492,
        (4, _, _) => 0xC299,
        (_, _, _) => 0xC200
    };

    let position = (0..4)
        .map(|i| gb.read_memory(base_addr + i))
        .filter(|&r| r <= 0x7F)
        .max();

    // TODO: detect HFF condition
    match position {
        Some(row) => Some(SongPosition { row, end: false }),
        None => None
    }
}

fn parse_title_bytes(title_bytes: &[u8]) -> Option<String> {
    let mut title_bytes = title_bytes.to_vec();
    if let Some(terminator) = title_bytes.iter().rposition(|&b| b != 0) {
        title_bytes.truncate(terminator + 1);
    }
    let title = match String::from_utf8(title_bytes) {
        Ok(title) => title,
        Err(_) => return None
    };

    if !title.starts_with("LSDj-v") {
        return None;
    }
    Some(title.replace("LSDj-v", ""))
}

fn get_running_lsdj_version(gb: &mut Gameboy) -> Option<String> {
    parse_title_bytes(&gb.game_title().into_bytes())
}

pub fn get_lsdj_version<P: AsRef<Path>>(rom_path: P) -> io::Result<Option<String>> {
    let mut rom = BufReader::new(File::open(rom_path)?);
    rom.seek(SeekFrom::Start(0x134))?;

    let mut title_bytes = vec![0u8; 11];
    rom.read_exact(&mut title_bytes)?;

    Ok(parse_title_bytes(&title_bytes))
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
        .map(|c| {
            let c = c.iter().take_while(|&&b| b != 0).cloned().collect();
            String::from_utf8(c).unwrap()
        })
        .filter(|c| !c.is_empty())
        .collect();

    Ok(Some(titles))
}
