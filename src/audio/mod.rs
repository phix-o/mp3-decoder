mod frame;
mod header;

use self::frame::MP3AudioFrame;
use std::io::Error;

pub fn parse_audio_frames(bytes: &[u8]) -> Result<Vec<MP3AudioFrame>, Error> {
    let mut frames = Vec::new();
    let mut current_index = 0;

    while current_index < bytes.len() {
        let frame = MP3AudioFrame::from_bytes(&bytes[current_index..])?;
        //println!("{current_index} {} {}", bytes.len(), frame.size);
        current_index += frame.frame_length as usize;

        frames.push(frame);

        if current_index > 1000 {
            break;
        }
    }

    Ok(frames)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_audio_frames() {
        // Example MP3 data with a single frame (replace with actual MP3 data)
        let mp3_data = [0xFF, 0xFA, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00];
        let frames = parse_audio_frames(&mp3_data).unwrap();

        assert_eq!(frames.len(), 1);
        // assert_eq!(frames[0].frame_length, 4);
    }
}
