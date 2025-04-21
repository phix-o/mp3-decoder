use std::io::Error;

use super::header::MP3AudioFrameHeader;

pub struct MP3AudioFrame<'a> {
    pub header: MP3AudioFrameHeader,
    pub data: &'a [u8],

    /// The total size of this frame
    pub frame_length: u32,
}
impl<'a> MP3AudioFrame<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        let header = MP3AudioFrameHeader::from_bytes(&bytes[..4].try_into().unwrap())?;

        let padding = match header.has_padding {
            true => 1,
            false => 0,
        };

        let samples_per_frame = header.layer.get_samples_per_frame();
        let frame_length = (samples_per_frame as u32)
            * ((header.bitrate as u32) / (header.sample_rate as u32))
            + padding;

        println!(
            "Frame length {frame_length}: {samples_per_frame} {} {} {padding}",
            header.bitrate, header.sample_rate
        );
        Ok(Self {
            header,
            frame_length,
            data: &bytes[4..],
        })
    }
}
impl<'a> std::fmt::Display for MP3AudioFrame<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MP3AudioFrame {{ frame_length: {}, header: {:?} }}",
            self.frame_length, self.header
        )
    }
}
