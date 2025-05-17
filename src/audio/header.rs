use std::{io::{Error, ErrorKind}, usize};

use crate::utils::HexSlice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MPEGVersion {
    /// MPEG-1 (ISO/IEC 11172-3, most common)
    Mpeg1,
    /// MPEG-2 (ISO/IEC 13818-3)
    Mpeg2,
    /// MPEG-2.5 (unofficial extension)
    Mpeg2_5,
}
impl MPEGVersion {
    /// Parses MPEG version from the 2-bit value in the frame header
    ///
    /// 00=MPEG-2.5, 01=reserved, 10=MPEG-2, 11=MPEG-1.
    pub fn from_bits(bits: u8) -> Result<Self, Error> {
        println!("Bits: {:02b}", bits);
        match bits {
            0b00 => Ok(Self::Mpeg2_5),
            0b10 => Ok(Self::Mpeg2),
            0b11 => Ok(Self::Mpeg1),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Expected 2-bit integer. Received {:08b}", bits),
            )), // Shouldn't happen as we're working with 2 bits
        }
    }

    /// Returns the bitrate (in bps) given the layer_name and bitrate index
    pub fn get_bitrate(&self, layer: Layer, index: u8) -> Result<Option<u32>, Error> {
        let table = match (self, layer) {
            (Self::Mpeg1, Layer::Layer1) => &[
                32, 64, 36, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
            ],
            (Self::Mpeg1, Layer::Layer2) => &[
                32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
            ],
            (Self::Mpeg1, Layer::Layer3) => &[
                32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
            ],
            (_, Layer::Layer1) => &[
                32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
            ],
            (_, _) => &[8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        };

        match index {
            0b0000 => Ok(None), // Free

            // Subtract 1 because the table entry at index 0 corresponds to bitrate index 1, ie
            // bitrate index 0 is not accounted for in the table
            0b0001..=0b1110 => Ok(Some(table[(index as usize) - 1] * 1000)),

            0b1111 => Ok(None), // Invalid. Should throw an error maybe?
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Expected 4-bit index. Received {:08b}", index),
            )),
        }
    }

    /// Returns the sampling rate (in Hertz) given the sampling rate index
    pub fn get_sampling_rate(&self, index: u8) -> Result<u16, Error> {
        let table: &[u16; 3] = match self {
            Self::Mpeg1 => &[44100, 48000, 32000],
            Self::Mpeg2 => &[22050, 24000, 16000],
            Self::Mpeg2_5 => &[11025, 12000, 8000],
        };

        match index {
            0b00..=0b10 => Ok(table[index as usize]),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Only valid integers, from 0 to 3 (inclusive), are allowed. Received {index}",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Layer1,
    Layer2,
    Layer3,
}
impl Layer {
    pub fn from_bits(bits: u8) -> Result<Self, Error> {
        match bits {
            0b01 => Ok(Self::Layer3),
            0b10 => Ok(Self::Layer2),
            0b11 => Ok(Self::Layer1),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Only valid integers, from 0 to 3 (inclusive), are allowed. Received {bits}",
            )),
        }
    }

    pub fn get_samples_per_frame(&self) -> u16 {
        match self {
            Self::Layer1 => 384,
            _ => 1152,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    Stereo,
    JointStereo,
    SingleChannel,
    DualChannel,
}
impl ChannelMode {
    pub fn from_bits(bits: u8) -> Result<Self, Error> {
        match bits {
            0b00 => Ok(Self::Stereo),
            0b01 => Ok(Self::JointStereo),
            0b10 => Ok(Self::DualChannel),
            0b11 => Ok(Self::SingleChannel),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Expected 2-bit number. Received {:08b}", bits),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeExtension {
    // Intensity Stereo = off, MS Stereo = off
    Mode1,

    // Intensity Stereo = on, MS Stereo = off
    Mode2,

    // Intensity Stereo = off, MS Stereo = on
    Mode3,

    // Intensity Stereo = on, MS Stereo = on
    Mode4,
}
impl ModeExtension {
    pub fn from_bits(bits: u8) -> Result<Self, Error> {
        match bits {
            0b00 => Ok(Self::Mode1),
            0b01 => Ok(Self::Mode2),
            0b10 => Ok(Self::Mode3),
            0b11 => Ok(Self::Mode4),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Expected a 2-bit number. Received {:08b}", bits),
            )),
        }
    }
}

#[derive(Debug)]
pub struct MP3AudioFrameHeader {
    mpeg_version: MPEGVersion,
    pub layer: Layer,
    has_crc: bool,

    /// The bitrate in bps
    pub bitrate: u32,

    /// Sampling rate in Hertz
    pub sample_rate: u16,

    pub has_padding: bool,
    channel_mode: ChannelMode,

    /// The state of the stereo intensity and mid-side (MS) stereo.
    ///
    /// Only used when channel_mode is `ChannelMode::JointStereo`
    mode_extension: ModeExtension,

    is_copywrighted: bool,

    /// Whether this bitstream is original or a copy
    is_original: bool,

    // misc
    duration_per_frame: f64,
}
impl MP3AudioFrameHeader {
    pub fn from_bytes(bytes: &[u8; 4]) -> Result<Self, Error> {
        let data = u32::from_be_bytes(*bytes);
        println!("{data}: {}", HexSlice::new(bytes));
        let mut bit_position = 32;

        bit_position -= 11; // First 11 bits (Supports MPEG2.5)
        let sync_word_bits = data >> bit_position;
        let sync_word_valid = 0x7FF;
        if sync_word_bits & sync_word_valid != sync_word_valid {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected 0xFFF for sync word. Received 0x{:X}",
                    sync_word_bits
                ),
            ));
        }

        bit_position -= 2; // Next 2 bits
        let mpeg_version_bits = ((data >> bit_position) & 0b11) as u8;
        let mpeg_version = MPEGVersion::from_bits(mpeg_version_bits)?;

        bit_position -= 2; // Next 2 bits
        let layer_bits = ((data >> bit_position) & 0b11) as u8;
        let layer = Layer::from_bits(layer_bits)?;

        bit_position -= 1; // Next bit
        let has_crc = ((data >> bit_position) & 0b1) == 0;

        bit_position -= 4; // Next 4 bits
        let bitrate_index = ((data >> bit_position) & 0b1111) as u8;
        let bitrate_from_index = mpeg_version.get_bitrate(layer, bitrate_index)?;

        bit_position -= 2; // Next 2 bits
        let sampling_rate_index = ((data >> bit_position) & 0b11) as u8;
        let sample_rate = mpeg_version.get_sampling_rate(sampling_rate_index)?;

        bit_position -= 1; // Next bit
        let padding = (data >> bit_position) & 0b1;
        let has_padding = padding == 1;

        // Skip next bit (Private bit)
        bit_position -= 1;

        bit_position -= 2; // Next 2 bits;
        let channel_mode_bits = ((data >> bit_position) & 0b11) as u8;
        let channel_mode = ChannelMode::from_bits(channel_mode_bits)?;

        bit_position -= 2; // Next 2 bits;
        let mode_extension_bits = ((data >> bit_position) & 0b11) as u8;
        let mode_extension = ModeExtension::from_bits(mode_extension_bits)?;

        bit_position -= 1; // Next bit
        let is_copywrighted = ((data >> bit_position) & 0b1) == 1;
        bit_position -= 1; // Next bit
        let is_original = ((data >> bit_position) & 0b1) == 1;

        // Ignore the emphasis

        let bitrate = bitrate_from_index.unwrap();
        let duration_per_frame = layer.get_samples_per_frame() as f64 / sample_rate as f64;

        Ok(Self {
            mpeg_version,
            layer,
            has_crc,
            bitrate,
            sample_rate,
            has_padding,
            channel_mode,
            mode_extension,
            is_copywrighted,
            is_original,
            duration_per_frame,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp3_audio_frame_header_from_bytes() {
        // Example header bytes (replace with actual valid header bytes)
        let header_bytes = [0xFF, 0xFB, 0x90, 0x44];
        let header = MP3AudioFrameHeader::from_bytes(&header_bytes).unwrap();

        assert_eq!(header.mpeg_version, MPEGVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.has_crc, false);
        assert_eq!(header.bitrate, 128);
        assert_eq!(header.sample_rate, 44100);
        assert_eq!(header.has_padding, false);
        assert_eq!(header.channel_mode, ChannelMode::JointStereo);
        assert_eq!(header.is_copywrighted, false);
        assert_eq!(header.is_original, true);
    }

    #[test]
    fn test_mp3_audio_frame_header_from_bytes_case2() {
        // Example header bytes
        let header_bytes = [0xFF, 0xFB, 0x90, 0x64];
        let header = MP3AudioFrameHeader::from_bytes(&header_bytes).unwrap();

        assert_eq!(header.mpeg_version, MPEGVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.has_crc, false);
        assert_eq!(header.bitrate, 128);
        assert_eq!(header.sample_rate, 44100);
        assert_eq!(header.has_padding, false);
        assert_eq!(header.channel_mode, ChannelMode::JointStereo);
        assert_eq!(header.is_copywrighted, false);
        assert_eq!(header.is_original, true);
    }

    #[test]
    fn test_invalid_sync_word() {
        let header_bytes = [0x00, 0x00, 0x00, 0x00];
        let result = MP3AudioFrameHeader::from_bytes(&header_bytes);
        assert!(result.is_err());
    }
}

