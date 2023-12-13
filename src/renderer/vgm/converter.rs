use anyhow::{Result, bail, Context};
use super::vgm::{Vgm, VgmIterItem};

const WAIT_CMD: u8 = 0x80;
const NEXT_BANK_CMD: u8 = 0xA0;
const WRITE_HRAM_CMD: u8 = 0xB0;
const LOOP_CMD: u8 = 0xC0;
const END_SONG_CMD: u8 = 0xD0;

const VGM_SAMPLE_RATE: f32 = 44100.0;
const TMA_RATE_0: u32 = 4096;

pub struct PegmodeEngineData {
    pub banks: Vec<Vec<u8>>,
    pub loop_data: Option<(usize, usize)>
}

pub fn samples_to_frames(samples: u32, engine_rate: u32) -> u32 {
    let frames = (samples as f32 * engine_rate as f32) / VGM_SAMPLE_RATE;
    if 0.0 < frames.fract() && frames.fract() < 0.98 {
        println!("Warning: calculated frame length {} is not near a whole number, timing may be inaccurate", frames)
    }
    frames.round() as u32
}

fn calculate_tma_modulo(tma_rate: u32, engine_rate: u32) -> i32 {
    0xFF - (tma_rate as f32 / engine_rate as f32).round() as i32
}

fn vgm_to_engine_format(vgm: &mut Vgm, for_2x: bool, engine_rate: u32) -> Result<PegmodeEngineData> {
    let mut result = PegmodeEngineData {
        banks: Vec::new(),
        loop_data: None
    };
    let mut current_bank: Vec<u8> = Vec::new();

    let (_clock, is_2x) = vgm.lr35902_clock().context("VGM does not have a Game Boy!")?;

    for command in vgm.iter() {
        if current_bank.len() >= 0x3FFC {
            current_bank.push(NEXT_BANK_CMD);
            result.banks.push(current_bank.clone());
            current_bank.clear();
        }

        match command {
            VgmIterItem::HitLoopOffset => {
                let loop_bank = result.banks.len() + 1;
                let loop_addr = current_bank.len() + 0x4000;
                result.loop_data = Some((loop_bank, loop_addr));
            }
            VgmIterItem::WaitCommand(samples) => {
                let mut frames = samples_to_frames(samples as u32, engine_rate);
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
            VgmIterItem::WriteLR35902RegCommand(cmd_is_2x, addr, val) => {
                if cmd_is_2x && !is_2x {
                    bail!("Encountered 2x write in non-2x VGM");
                }
                if cmd_is_2x == for_2x {
                    let h_addr = (addr & 0xFF) as u8;
                    current_bank.push(h_addr);
                    current_bank.push(val);
                } else {
                    // Dummy write to $FF50 (boot ROM unmap register) to maintain sync
                    current_bank.push(0x50);
                    current_bank.push(0x01);
                }
            }
            VgmIterItem::InvalidCommand(cmd) => {
                bail!("Invalid/unsupported VGM command {:02X}!", cmd);
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
    buf.resize(0x20, 0);
    buf
}

pub fn vgm_to_gbs(vgm: &mut Vgm, for_2x: bool, engine_rate: u32, tma_offset: i32) -> Result<Vec<u8>> {
    let engine_data = vgm_to_engine_format(vgm, for_2x, engine_rate)?;

    let mut patch_rom = include_bytes!("patch_rom.gb").to_vec();
    patch_rom.resize(patch_rom.len() + (engine_data.banks.len() + 1) * 0x4000, 0);

    for (i, bank) in engine_data.banks.iter().enumerate() {
        let bank_offset = 0x4000 * (i + 1);
        patch_rom[bank_offset..(bank_offset + bank.len())].copy_from_slice(&bank);
    }
    if let Some((loop_bank, loop_addr)) = engine_data.loop_data {
        patch_rom[0x3FFC..0x3FFE].copy_from_slice(&u16::to_le_bytes(loop_addr as u16));
        patch_rom[0x3FFE..0x4000].copy_from_slice(&u16::to_le_bytes(loop_bank as u16));
    }

    let (tma, tac) = match engine_rate {
        60 => (tma_offset as u8, 0u8),
        _ => ((calculate_tma_modulo(TMA_RATE_0, engine_rate) + tma_offset) as u8, 4u8)
    };

    patch_rom[0x3FFA] = tma; // Engine TMA
    patch_rom[0x3FFB] = tac; // Engine TAC

    let mut gbs: Vec<u8> = Vec::with_capacity(patch_rom.len() - 0x3E80);
    gbs.extend(b"GBS"); // Magic
    gbs.push(1); // Version
    gbs.push(1); // Song count
    gbs.push(1); // First song index
    gbs.extend(&u16::to_le_bytes(0x3EF0)); // Load address
    gbs.extend(&u16::to_le_bytes(0x3EF0)); // Init address
    gbs.extend(&u16::to_le_bytes(0x3F26)); // Play address
    gbs.extend(&u16::to_le_bytes(0xFFFE)); // Stack pointer
    gbs.push(tma); // Timer TMA
    gbs.push(tac); // Timer TAC

    debug_assert_eq!(gbs.len(), 0x10);

    if let Some(gd3) = vgm.gd3_metadata() {
        gbs.extend(metadata_string(&gd3.title));
        gbs.extend(metadata_string(&gd3.author));
        gbs.extend(metadata_string(&gd3.game));
    } else {
        gbs.extend(metadata_string("<?>"));
        gbs.extend(metadata_string("<?>"));
        gbs.extend(metadata_string("<?>"));
    }

    debug_assert_eq!(gbs.len(), 0x70);

    gbs.extend_from_slice(&patch_rom[0x3EF0..]);

    Ok(gbs)
}
