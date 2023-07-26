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
    pub fn new(data: &[u8]) -> Result<Self, String> {
        if &data[0..4] != b"Gd3 " {
            return Err("Invalid GD3 metadata".to_string());
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
            .map_err(|e| e.to_string())?
            .into_bytes();
        let mut string_table = string_table_bytes.split(|c| *c == 0);

        // TODO handle errors instead of panicking
        result.title = String::from_utf8(string_table.next().unwrap().to_vec()).unwrap();
        string_table.next().unwrap();
        result.game = String::from_utf8(string_table.next().unwrap().to_vec()).unwrap();
        string_table.next().unwrap();
        result.system = String::from_utf8(string_table.next().unwrap().to_vec()).unwrap();
        string_table.next().unwrap();
        result.author = String::from_utf8(string_table.next().unwrap().to_vec()).unwrap();
        string_table.next().unwrap();
        string_table.next().unwrap();
        result.ripper = String::from_utf8(string_table.next().unwrap().to_vec()).unwrap();
        string_table.next().unwrap();

        Ok(result)
    }
}
