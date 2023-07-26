use super::vgm::{Vgm, VgmIterItem};

const WAIT_CMD: u8 = 0x80;
const NEXT_BANK_CMD: u8 = 0xA0;
const WRITE_HRAM_CMD: u8 = 0xB0;
const LOOP_CMD: u8 = 0xC0;
const END_SONG_CMD: u8 = 0xD0;

pub struct PegmodeEngineData {
    pub banks: Vec<Vec<u8>>,
    pub loop_data: Option<(usize, usize)>
}

fn vgm_to_engine_format(vgm: &mut Vgm) -> Result<PegmodeEngineData, String> {
    let mut result = PegmodeEngineData {
        banks: Vec::new(),
        loop_data: None
    };
    let mut current_bank: Vec<u8> = Vec::new();

    for command in vgm {
        if current_bank.len() >= 0x3FFC {
            current_bank.push(NEXT_BANK_CMD);
            result.banks.push(current_bank.clone());
            current_bank.clear();
        }

        println!("{:?}", command);

        match command {
            VgmIterItem::HitLoopOffset => {
                let loop_bank = result.banks.len() + 1;
                let loop_addr = current_bank.len() + 0x4000;
                result.loop_data = Some((loop_bank, loop_addr));
            }
            VgmIterItem::WaitCommand(samples) => {
                let mut frames = ((60.0 * samples as f32) / 44100.0).round() as usize;
                while frames > 0xFF {
                    current_bank.push(WAIT_CMD);
                    current_bank.push(0xFF);
                    frames -= 0xFF;
                }
                current_bank.push(WAIT_CMD);
                current_bank.push(frames as u8);
            }
            VgmIterItem::DataBlock(_, block) => {
                // Used for Deflemask sync writes
                current_bank.push(WRITE_HRAM_CMD);
                current_bank.push(0x80);
                current_bank.push(block[0]);
            }
            VgmIterItem::WriteLR35902RegCommand(addr, val) => {
                let h_addr = (addr & 0xFF) as u8;
                current_bank.push(h_addr);
                current_bank.push(val);
            }
            VgmIterItem::InvalidCommand(cmd) => {
                return Err(format!("Invalid/unsupported VGM command {:02X}!", cmd))
            }
        }
    }

    if result.loop_data.is_some() {
        current_bank.push(LOOP_CMD);
    } else {
        current_bank.push(END_SONG_CMD);
    }

    result.banks.push(current_bank);
    Ok(result)
}

fn metadata_string(s: &str) -> Vec<u8> {
    let mut buf = s.to_owned().into_bytes();
    while buf.len() < 32 {
        buf.push(0);
    }
    buf.truncate(32);
    buf
}

pub fn vgm_to_gbs(vgm: &mut Vgm) -> Result<(Vec<u8>, PegmodeEngineData), String> {
    let mut patch_rom = include_bytes!("patch_rom.gb").to_vec();

    let engine_data = vgm_to_engine_format(vgm)?;
    for (i, bank) in engine_data.banks.iter().enumerate() {
        let bank_offset = 0x4000 * (i + 1);
        patch_rom[bank_offset..(bank_offset + bank.len())].copy_from_slice(&bank);
    }
    if let Some((loop_bank, loop_addr)) = engine_data.loop_data {
        patch_rom[0x3FFC..0x3FFE].copy_from_slice(&u16::to_le_bytes(loop_addr as u16));
        patch_rom[0x3FFE..0x4000].copy_from_slice(&u16::to_le_bytes(loop_bank as u16));
    }

    patch_rom[0x3FFA] = 0; // Engine TMA
    patch_rom[0x3FFB] = 0; // Engine TAC

    let mut gbs: Vec<u8> = b"GBS".to_vec(); // Magic
    gbs.push(1); // Version
    gbs.push(1); // Song count
    gbs.push(1); // First song index
    gbs.extend(&u16::to_le_bytes(0x3EF0)); // Load address
    gbs.extend(&u16::to_le_bytes(0x3EF0)); // Init address
    gbs.extend(&u16::to_le_bytes(0x3F26)); // Play address
    gbs.extend(&u16::to_le_bytes(0xFFFE)); // Stack pointer
    gbs.push(0); // Timer TMA
    gbs.push(0); // Timer TAC

    assert_eq!(gbs.len(), 0x10);

    if let Some(gd3) = vgm.gd3_metadata() {
        gbs.extend(metadata_string(&gd3.title));
        gbs.extend(metadata_string(&gd3.author));
        gbs.extend(metadata_string(&gd3.game));
    } else {
        gbs.extend(metadata_string("<?>"));
        gbs.extend(metadata_string("<?>"));
        gbs.extend(metadata_string("<?>"));
    }

    assert_eq!(gbs.len(), 0x70);

    gbs.extend_from_slice(&patch_rom[0x3EF0..]);
    Ok((gbs, engine_data))
}
