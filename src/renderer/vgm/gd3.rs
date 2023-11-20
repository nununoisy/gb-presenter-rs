use anyhow::{Result, Context, bail};
use std::slice;

#[derive(Clone, Default)]
pub struct Gd3 {
    pub version: u32,
    pub title: String,
    pub game: String,
    pub system: String,
    pub author: String,
    pub ripper: String,
    pub notes: String
}

impl Gd3 {
    pub fn new(data: &[u8]) -> Result<Self> {
        if &data[0..4] != b"Gd3 " {
            bail!("Invalid GD3 metadata!");
        }

        let mut result = Self::default();

        let mut buf = [0u8; 4];
        buf.copy_from_slice(&data[0x4..0x8]);
        result.version = u32::from_le_bytes(buf);
        buf.copy_from_slice(&data[0x8..0xC]);
        let data_size = u32::from_le_bytes(buf) as usize;

        let utf16_string_slice = unsafe {
            slice::from_raw_parts(
                (&data[0xC..(0xC + data_size)]).as_ptr() as *const u16,
                data_size
            )
        };
        let string_table_bytes = String::from_utf16(utf16_string_slice)
            .context("Invalid GD3 string table!")?
            .into_bytes();
        let mut string_table = string_table_bytes.split(|c| *c == 0);

        result.title = String::from_utf8(string_table.next().context("Missing title!")?.to_vec())?;

        string_table.next().context("Missing original title!")?;

        result.game = String::from_utf8(string_table.next().context("Missing game name!")?.to_vec())?;

        string_table.next().context("Missing original game name!")?;

        result.system = String::from_utf8(string_table.next().context("Missing system name!")?.to_vec())?;

        string_table.next().context("Missing original system name!")?;

        result.author = String::from_utf8(string_table.next().context("Missing author name!")?.to_vec())?;

        string_table.next().context("Missing original author name!")?;

        string_table.next().context("Missing release date!")?;

        result.ripper = String::from_utf8(string_table.next().context("Missing ripper name!")?.to_vec())?;

        string_table.next().context("Missing notes!")?;

        Ok(result)
    }
}
