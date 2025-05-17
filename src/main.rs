mod audio;
mod metadata;
mod utils;

use std::fs::File;
use std::io::{Error, Read};

use crate::audio::parse_audio_frames;
use crate::metadata::header::ID3v2Header;
use crate::utils::HexSlice;

fn main() -> Result<(), Error> {
    let file_path = "./assets/sample_1.mp3";

    let mut file = File::open(&file_path)?;
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer)?;

    println!("Read {} bytes", buffer.len());

    let header = ID3v2Header::from_bytes(&buffer)?;
    println!("Header info");
    println!("{:?}", header);

    let start_of_audio = header.size;
    let audio_frames_bytes = &buffer[(start_of_audio as usize)..];
    println!(
        "Audio frames bytes: {}",
        HexSlice::new(&audio_frames_bytes[0..20])
    );

    let audio_frames = parse_audio_frames(&audio_frames_bytes)?;
    println!("\nAudio Frames: {}", audio_frames.len());
    for frame in audio_frames.iter().take(3) {
        println!("{frame}");
    }

    Ok(())
}
