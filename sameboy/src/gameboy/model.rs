use super::Gameboy;
use sameboy_sys::{GB_get_model, GB_model_t, GB_model_t_GB_MODEL_DMG_B, GB_model_t_GB_MODEL_CGB_0, GB_model_t_GB_MODEL_CGB_A, GB_model_t_GB_MODEL_CGB_B, GB_model_t_GB_MODEL_CGB_C, GB_model_t_GB_MODEL_CGB_D, GB_model_t_GB_MODEL_CGB_E, GB_model_t_GB_MODEL_SGB_NTSC, GB_model_t_GB_MODEL_SGB_PAL, GB_model_t_GB_MODEL_SGB_PAL_NO_SFC, GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC, GB_model_t_GB_MODEL_SGB2, GB_model_t_GB_MODEL_SGB2_NO_SFC, GB_model_t_GB_MODEL_MGB, GB_model_t_GB_MODEL_AGB, GB_model_t_GB_MODEL_GBP};

#[derive(Copy, Clone, PartialEq)]
pub enum Revision {
    Rev0,
    RevA,
    RevB,
    RevC,
    RevD,
    RevE
}

#[derive(Copy, Clone, PartialEq)]
pub enum VideoStandard {
    NTSC,
    PAL
}

#[derive(Copy, Clone, PartialEq)]
pub enum Model {
    /// The original "gray brick" Game Boy.
    DMG(Revision),
    /// The Game Boy Color.
    CGB(Revision),
    /// The Super Game Boy cartridge.
    /// Includes info about video standard (NTSC/PAL) and
    /// whether it is attached to a virtual Super Famicom.
    SGB(VideoStandard, bool),
    /// The Super Game Boy 2 cartridge.
    /// Includes info about whether it is attached to a virtual
    /// Super Famicom. Since it is Japan exclusive, it is always NTSC.
    SGB2(bool),
    /// The Game Boy Pocket and GameBoy Light.
    MGB,
    /// The backwards-compatiblity mode of the Game Boy Advance.
    AGB,
    /// The backwards-compatiblity mode of the Game Boy Player.
    GBP
}

impl From<GB_model_t> for Model {
    fn from(value: GB_model_t) -> Self {
        match value {
            GB_model_t_GB_MODEL_DMG_B => Self::DMG(Revision::RevB),
            GB_model_t_GB_MODEL_CGB_0 => Self::CGB(Revision::Rev0),
            GB_model_t_GB_MODEL_CGB_A => Self::CGB(Revision::RevA),
            GB_model_t_GB_MODEL_CGB_B => Self::CGB(Revision::RevB),
            GB_model_t_GB_MODEL_CGB_C => Self::CGB(Revision::RevC),
            GB_model_t_GB_MODEL_CGB_D => Self::CGB(Revision::RevD),
            GB_model_t_GB_MODEL_CGB_E => Self::CGB(Revision::RevE),
            GB_model_t_GB_MODEL_SGB_NTSC => Self::SGB(VideoStandard::NTSC, true),
            GB_model_t_GB_MODEL_SGB_PAL => Self::SGB(VideoStandard::PAL, true),
            GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC => Self::SGB(VideoStandard::NTSC, false),
            GB_model_t_GB_MODEL_SGB_PAL_NO_SFC => Self::SGB(VideoStandard::PAL, false),
            GB_model_t_GB_MODEL_SGB2 => Self::SGB2(true),
            GB_model_t_GB_MODEL_SGB2_NO_SFC => Self::SGB2(false),
            GB_model_t_GB_MODEL_MGB => Self::MGB,
            GB_model_t_GB_MODEL_AGB => Self::AGB,
            GB_model_t_GB_MODEL_GBP => Self::GBP,
            _ => unreachable!("Invalid GB_model_t value")
        }
    }
}

impl From<Model> for GB_model_t {
    fn from(value: Model) -> Self {
        match value {
            Model::DMG(Revision::RevB) => GB_model_t_GB_MODEL_DMG_B,
            Model::CGB(Revision::Rev0) => GB_model_t_GB_MODEL_CGB_0,
            Model::CGB(Revision::RevA) => GB_model_t_GB_MODEL_CGB_A,
            Model::CGB(Revision::RevB) => GB_model_t_GB_MODEL_CGB_B,
            Model::CGB(Revision::RevC) => GB_model_t_GB_MODEL_CGB_C,
            Model::CGB(Revision::RevD) => GB_model_t_GB_MODEL_CGB_D,
            Model::CGB(Revision::RevE) => GB_model_t_GB_MODEL_CGB_E,
            Model::SGB(VideoStandard::NTSC, true) => GB_model_t_GB_MODEL_SGB_NTSC,
            Model::SGB(VideoStandard::PAL, true) => GB_model_t_GB_MODEL_SGB_PAL,
            Model::SGB(VideoStandard::NTSC, false) => GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC,
            Model::SGB(VideoStandard::PAL, false) => GB_model_t_GB_MODEL_SGB_PAL_NO_SFC,
            Model::SGB2(true) => GB_model_t_GB_MODEL_SGB2,
            Model::SGB2(false) => GB_model_t_GB_MODEL_SGB2_NO_SFC,
            Model::MGB => GB_model_t_GB_MODEL_MGB,
            Model::AGB => GB_model_t_GB_MODEL_AGB,
            Model::GBP => GB_model_t_GB_MODEL_GBP,
            _ => panic!("Invalid Model conversion")
        }
    }
}

impl Gameboy {
    /// Get the emulated Game Boy model.
    pub fn model(&mut self) -> Model {
        unsafe {
            GB_get_model(self.as_mut_ptr()).into()
        }
    }

    /// Helper to get the default filename for the bootrom for this Gameboy's model.
    pub fn preferred_boot_rom(&mut self) -> String {
        match self.model() {
            Model::DMG(_) => "dmg_boot.bin".to_string(),
            Model::CGB(Revision::Rev0) => "cgb0_boot.bin".to_string(),
            Model::CGB(_) => "cgb_boot.bin".to_string(),
            Model::SGB(_, _) => "sgb_boot.bin".to_string(),
            Model::SGB2(_) => "sgb2_boot.bin".to_string(),
            Model::MGB => "mgb_boot.bin".to_string(),
            Model::AGB | Model::GBP => "agb_boot.bin".to_string()
        }
    }
}
