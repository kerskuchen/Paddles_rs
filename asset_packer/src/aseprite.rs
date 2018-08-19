use bincode;
use std::fs::File;

const MAGIC_NUMBER_HEADER: u16 = 0xA5E0;
const MAGIC_NUMBER_FRAME: u16 = 0xF1FA;

const COLOR_DEPTH_RGBA: u16 = 32;
const COLOR_DEPTH_GREYSCALE: u16 = 16;
const COLOR_DEPTH_INDEXED: u16 = 8;

const CHUNKTYPE_OLD_PALETTE0: u16 = 0x0004;
const CHUNKTYPE_OLD_PALETTE1: u16 = 0x0011;
const CHUNKTYPE_LAYER: u16 = 0x2004;
const CHUNKTYPE_CELL: u16 = 0x2005;
const CHUNKTYPE_CELL_EXTRA: u16 = 0x2006;
const CHUNKTYPE_MASK: u16 = 0x2016; // Deprecated
const CHUNKTYPE_PATH: u16 = 0x2017; // Unused
const CHUNKTYPE_FRAME_TAGS: u16 = 0x2018;
const CHUNKTYPE_PALETTE: u16 = 0x2019;
const CHUNKTYPE_USER_DATA: u16 = 0x2020;
const CHUNKTYPE_SLICE: u16 = 0x2022;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct Header {
    pub file_size: u32,
    pub magic_number: u16,
    pub num_frames: u16,
    pub image_width_in_pixels: u16,
    pub image_height_in_pixels: u16,
    pub color_depth: u16,
    pub flags: u32,
    pub frame_default_duration_in_ms: u16,
    pub _set_to_be_zero0: u32,
    pub _set_to_be_zero1: u32,
    pub indexed_color_transparent: u8,
    pub _ignored: [u8; 3],
    pub num_colors: u16,
    pub pixel_width: u8,
    pub pixel_height: u8,
    pub _reserved0: [u8; 32],
    pub _reserved1: [u8; 32],
    pub _reserved2: [u8; 28],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct Frame {
    pub num_bytes: u32,
    pub magic_number: u16,
    pub num_chunks: u16,
    pub frame_duration_in_ms: u16,
    pub _reserved: [u8; 2],
    pub num_chunks_additional: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct ChunkHeader {
    pub chunk_size: u32,
    pub chunk_type: u16,
}

pub fn test_read_aseprite_file() {
    let mut aseprite_file =
        File::open("assets/images/test.aseprite").expect("Could not load aseprite file");

    let header: Header =
        bincode::deserialize_from(&mut aseprite_file).expect("Could not deserialize header");
    assert_eq!(header.magic_number, MAGIC_NUMBER_HEADER);

    let frame: Frame = bincode::deserialize_from(&mut aseprite_file).expect("Could not frame info");
    println!("{:#?}", frame);
    assert_eq!(frame.magic_number, MAGIC_NUMBER_FRAME);

    let chunk_header: ChunkHeader =
        bincode::deserialize_from(&mut aseprite_file).expect("Could not chunk header");

    println!("{:#?}", chunk_header);
}
