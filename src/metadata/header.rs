use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq)]
pub enum ID3v2MetadataFrameID {
    Title,
    Artist,
    Album,
    Year,
    Comment,
    TrackNumber,
    Genre,
    Txxx,
    Custom(Vec<u8>), // For non-standard frames
}

impl ID3v2MetadataFrameID {
    pub fn to_bytes(&self) -> &[u8] {
        match self {
            ID3v2MetadataFrameID::Title => b"TIT2",
            ID3v2MetadataFrameID::Artist => b"TPE1",
            ID3v2MetadataFrameID::Album => b"TALB",
            ID3v2MetadataFrameID::Year => b"TYER",
            ID3v2MetadataFrameID::Comment => b"COMM",
            ID3v2MetadataFrameID::TrackNumber => b"TRCK",
            ID3v2MetadataFrameID::Genre => b"TCON",
            ID3v2MetadataFrameID::Txxx => b"TXXX",
            ID3v2MetadataFrameID::Custom(bytes) => bytes.as_slice(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"TIT2" => Some(ID3v2MetadataFrameID::Title),
            b"TPE1" => Some(ID3v2MetadataFrameID::Artist),
            b"TALB" => Some(ID3v2MetadataFrameID::Album),
            b"TYER" => Some(ID3v2MetadataFrameID::Year),
            b"COMM" => Some(ID3v2MetadataFrameID::Comment),
            b"TRCK" => Some(ID3v2MetadataFrameID::TrackNumber),
            b"TCON" => Some(ID3v2MetadataFrameID::Genre),
            b"TXXX" => Some(ID3v2MetadataFrameID::Txxx),
            _ => Some(ID3v2MetadataFrameID::Custom(bytes.to_vec())),
        }
    }
}

#[derive(Debug)]
pub struct ID3v2MetadataFrame<'a> {
    /// 4-char identifier of this frame
    pub id: ID3v2MetadataFrameID,

    /// The size of this frame's data
    pub data_size: u32,

    /// The size including this frame's header
    pub size: u32,

    pub flags: u16,

    pub data: &'a [u8],
}
impl<'a> ID3v2MetadataFrame<'a> {
    /// Constructs an ID3v2MetadataFrame from bytes
    ///
    /// Expects that bytes[0] is the begining of this section, not the begining of the file
    pub fn from_bytes(bytes: &'a [u8], version: u8) -> Result<Self, Error> {
        if bytes.len() < 10 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Atleast 10 bytes are required",
            ));
        }

        let data_size = Self::parse_size(&[bytes[4], bytes[5], bytes[6], bytes[7]], version)?;
        let size = data_size + 10;
        Ok(Self {
            id: ID3v2MetadataFrameID::from_bytes(&bytes[..4]).unwrap(),
            data_size,
            size,
            flags: u16::from_be_bytes([bytes[8], bytes[9]]),
            data: &bytes[10..(size as usize)],
        })
    }

    fn parse_size(bytes: &[u8; 4], version: u8) -> Result<u32, Error> {
        if version != 3 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Only works with ID3v2.3",
            ));
        }

        let s = u32::from_be_bytes(*bytes);
        Ok(s)
    }
}

#[derive(Debug)]
pub struct ID3v2Header<'a> {
    pub version: u8,
    pub flags: u8,

    /// Size of the metadata after which the audio frames begin.
    ///
    /// Excludes the size of the tag itself (10 bytes)
    pub metadata_size: u32,

    /// The total size of the header
    pub size: u32,

    pub metadata_frames: Vec<ID3v2MetadataFrame<'a>>,
}

impl<'a> ID3v2Header<'a> {
    /// Constructs an ID3v2Header from bytes
    ///
    /// Structure:
    /// bytes\[0..3]     => represents the 'IDF' name in ASCII
    /// bytes\[4]        => the revision (minor) version. Always 0 in practice
    /// bytes\[5]        => flags (0x40 means has extra headers)
    /// bytes\[6..10]    => Size of header (minus 10 bytes for the actual header data)
    /// bytes\[10..size] => ID3v2 Metadata frames
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        if !Self::has_flag(&bytes) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "File doesn't have IDV3 header",
            ));
        }

        let metadata_size = Self::parse_size(&bytes);
        let size = metadata_size + 10;
        let version = bytes[3];
        let flags = bytes[5];
        let has_extended_header = Self::has_extended_header(flags);
        println!("Has extended header: {has_extended_header}");
        // TODO: Handle case where we have an extended header

        Ok(Self {
            version,
            flags,
            metadata_size,
            size,
            metadata_frames: Self::build_metadata_frames(&bytes[10..(size as usize)], version)?,
        })
    }

    fn has_flag(bytes: &[u8]) -> bool {
        bytes.len() >= 10 && &bytes[0..3] == b"ID3"
    }

    fn has_extended_header(flags: u8) -> bool {
        let mask = 0x40;
        (flags & mask) == mask
    }

    fn parse_size(bytes: &[u8]) -> u32 {
        ((bytes[6] as u32) << 21)
            | ((bytes[7] as u32) << 14)
            | ((bytes[8] as u32) << 7)
            | (bytes[9] as u32)
    }

    fn build_metadata_frames(bytes: &[u8], version: u8) -> Result<Vec<ID3v2MetadataFrame>, Error> {
        let mut frames = Vec::new();
        //println!("Bytes: {} {:?}", bytes.len(), bytes);

        let mut current_index = 0;
        while current_index < bytes.len() {
            let frame = ID3v2MetadataFrame::from_bytes(&bytes[current_index..], version)?;
            current_index += frame.size as usize;
            frames.push(frame);
        }

        Ok(frames)
    }
}
