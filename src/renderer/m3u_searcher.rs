use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use glob::{glob_with, MatchOptions};
use encoding_rs::{CoderResult, WINDOWS_1252, SHIFT_JIS};

fn read_m3u_file<P: AsRef<Path>>(m3u_path: P) -> Result<String, String> {
    let data = fs::read(m3u_path).map_err(|e| e.to_string())?;
    let mut result = String::with_capacity(data.len() * 4);

    let mut cp1252_decoder = WINDOWS_1252.new_decoder();
    let (coder_result, _bytes_read, did_replacements) = cp1252_decoder.decode_to_string(&data, &mut result, true);
    if coder_result != CoderResult::OutputFull && !did_replacements {
        return Ok(result);
    }

    result.clear();
    let mut shift_jis_decoder = SHIFT_JIS.new_decoder();
    let (coder_result, _bytes_read, did_replacements) = shift_jis_decoder.decode_to_string(&data, &mut result, true);
    if coder_result != CoderResult::OutputFull && !did_replacements {
        return Ok(result);
    }

    String::from_utf8(data).map_err(|e| e.to_string())
}

pub fn search<P: AsRef<Path>>(gbs_path: P) -> Result<HashMap<u8, (String, Option<Duration>)>, String> {
    let mut result: HashMap<u8, (String, Option<Duration>)> = HashMap::new();

    let gbs_filename = gbs_path.as_ref().file_name().unwrap().to_str().unwrap().to_string();

    let mut gbs_dir = gbs_path
        .as_ref()
        .parent()
        .ok_or("Invalid path".to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    gbs_dir.push("*.m3u");

    let mut gbs_dir = gbs_dir.to_str().unwrap().to_string();
    if gbs_dir.starts_with("\\\\?\\") {
        let _ = gbs_dir.drain(0..4);
    }

    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    for glob_entry in glob_with(&gbs_dir, options).map_err(|e| e.to_string())? {
        let m3u_path = glob_entry.map_err(|e| e.to_string())?;
        println!("Discovered M3U file: {}", m3u_path.file_name().unwrap().to_str().unwrap());

        for line in read_m3u_file(m3u_path).map_err(|e| e.to_string())?.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut components: Vec<String> = Vec::new();
            for raw_component in line.split(',') {
                if !components.is_empty() && components.last().unwrap().replace("\\\\", "").ends_with('\\') {
                    let _ = components.last_mut().unwrap().pop();
                    components.last_mut().unwrap().push(',');
                    components.last_mut().unwrap().push_str(&raw_component.replace("\\\\", "\\"));
                } else {
                    components.push(raw_component.replace("\\\\", "\\"));
                }
            }
            let mut component_iter = components.iter().cloned();

            let filename = component_iter.next().unwrap_or("".to_string());
            if filename.to_lowercase() != format!("{}::gbs", gbs_filename.to_lowercase()) {
                continue;
            }

            let index = match u8::from_str(&component_iter.next().unwrap_or("".to_string())) {
                Ok(i) => i,
                Err(e) => return Err(e.to_string())
            };

            let mut track_title = component_iter.next().unwrap_or("".to_string());
            if track_title.is_empty() {
                continue;
            } else if track_title.chars().count() > 60 {
                let new_len = track_title.char_indices().nth(57).map(|(i, _)| i).unwrap_or(track_title.len());
                track_title.truncate(new_len);
                track_title.push_str("...");
            }

            let duration_seconds = component_iter.next().unwrap_or("".to_string())
                .split(':')
                .fold(0u64, |acc, cur| {
                    let duration_component = u64::from_str(cur).unwrap_or_default();
                    (acc * 60) + duration_component
                });
            let duration = match duration_seconds {
                0 => None,
                _ => Some(Duration::from_secs(duration_seconds))
            };

            result.insert(index, (track_title, duration));
        }
    }

    Ok(result)
}
