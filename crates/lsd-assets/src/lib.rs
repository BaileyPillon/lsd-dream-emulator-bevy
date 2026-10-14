//! Bounded decoding of original disc assets and read-only reference validation.
mod effect_palette;
pub mod formats;
pub use effect_palette::load_effect_palette;
mod link_tables;
mod math_tables;
pub use link_tables::{OriginalLinkTables, load_link_tables};
pub use math_tables::load_math_tables;
mod scene_config;
mod stage_session;
mod streamed_stage;
pub use scene_config::{
    LayeredStage, load_layered_stage, load_scene_grid, load_stream_grid_tables, stage_map_path,
};
use sha2::{Digest, Sha256};
pub use stage_session::{StageLinkSession, StageSessionEvent};
use std::fmt;
pub use streamed_stage::StreamedStage;

pub const CANONICAL_SHA256: &str =
    "282308ade239ce61b5603651dbbab0476a681b5d3618e82c0d3c3a95316d302f";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsxExeHeader {
    pub entry_point: u32,
    pub global_pointer: u32,
    pub load_address: u32,
    pub text_bytes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    HeaderTruncated,
    BadMagic,
    PayloadTruncated,
    OutsideRam,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HeaderTruncated => "PS-X EXE header is truncated",
            Self::BadMagic => "input does not have PS-X EXE magic",
            Self::PayloadTruncated => "declared executable payload is truncated",
            Self::OutsideRam => "executable payload lies outside 2 MiB PlayStation RAM",
        })
    }
}

impl std::error::Error for InputError {}

impl PsxExeHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, InputError> {
        if bytes.len() < 0x800 {
            return Err(InputError::HeaderTruncated);
        }
        if &bytes[..8] != b"PS-X EXE" {
            return Err(InputError::BadMagic);
        }
        let word =
            |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let header = Self {
            entry_point: word(0x10),
            global_pointer: word(0x14),
            load_address: word(0x18),
            text_bytes: word(0x1C),
        };
        if u64::from(header.text_bytes) + 0x800 > bytes.len() as u64 {
            return Err(InputError::PayloadTruncated);
        }
        if u64::from(header.load_address & 0x1FFF_FFFF) + u64::from(header.text_bytes) > 0x20_0000 {
            return Err(InputError::OutsideRam);
        }
        Ok(header)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn is_canonical_reference(bytes: &[u8]) -> bool {
    sha256_hex(bytes) == CANONICAL_SHA256
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut data = vec![0; 0x804];
        data[..8].copy_from_slice(b"PS-X EXE");
        data[0x10..0x14].copy_from_slice(&0x8001_0000u32.to_le_bytes());
        data[0x18..0x1C].copy_from_slice(&0x8001_0000u32.to_le_bytes());
        data[0x1C..0x20].copy_from_slice(&4u32.to_le_bytes());
        data
    }

    #[test]
    fn parses_little_endian_header_without_accepting_it_as_the_game() {
        let data = fixture();
        let header = PsxExeHeader::parse(&data).unwrap();
        assert_eq!(header.entry_point, 0x8001_0000);
        assert_eq!(header.load_address, 0x8001_0000);
        assert_eq!(header.text_bytes, 4);
        assert!(!is_canonical_reference(&data));
    }

    #[test]
    fn rejects_short_headers_bad_magic_and_missing_payloads() {
        assert_eq!(PsxExeHeader::parse(&[]), Err(InputError::HeaderTruncated));
        let mut data = fixture();
        data[0] = 0;
        assert_eq!(PsxExeHeader::parse(&data), Err(InputError::BadMagic));
        let data = fixture();
        assert_eq!(
            PsxExeHeader::parse(&data[..0x800]),
            Err(InputError::PayloadTruncated)
        );
    }

    #[test]
    fn rejects_out_of_ram_extents_without_integer_overflow() {
        let mut data = fixture();
        data[0x18..0x1C].copy_from_slice(&0x801F_FFFEu32.to_le_bytes());
        assert_eq!(PsxExeHeader::parse(&data), Err(InputError::OutsideRam));
        data[0x1C..0x20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            PsxExeHeader::parse(&data),
            Err(InputError::PayloadTruncated)
        );
    }
}
