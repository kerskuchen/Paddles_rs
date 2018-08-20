use bincode::deserialize_from;
use std;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;

use failure;
use failure::{Error, ResultExt};

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
pub struct HeaderC {
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
pub struct FrameC {
    pub num_bytes: u32,
    pub magic_number: u16,
    pub num_chunks: u16,
    pub frame_duration_in_ms: u16,
    pub _reserved: [u8; 2],
    pub num_chunks_additional: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct ChunkHeaderC {
    pub chunk_size: u32,
    pub chunk_type: u16,
}

const CHUNKHEADER_SIZE: u32 = 6;

pub fn test_read_aseprite_file() {
    let mut file = File::open("assets/images/test.aseprite").expect("Could not load aseprite file");

    let header: HeaderC = deserialize_from(&mut file).expect("Could not deserialize header");
    assert_eq!(header.magic_number, MAGIC_NUMBER_HEADER);

    let frame: FrameC = deserialize_from(&mut file).expect("Could not deserialize frame info");
    assert_eq!(frame.magic_number, MAGIC_NUMBER_FRAME);

    // Palette chunk
    let chunk_header: ChunkHeaderC =
        deserialize_from(&mut file).expect("Could not deserialize chunk header");
    if chunk_header.chunk_type == CHUNKTYPE_PALETTE {
        read_chunk_palette(&mut file).expect("Could not read palette chunk")
    }

    // Old chunk
    let chunk_header: ChunkHeaderC =
        deserialize_from(&mut file).expect("Could not deserialize chunk header");
    if chunk_header.chunk_type == CHUNKTYPE_OLD_PALETTE0
        || chunk_header.chunk_type == CHUNKTYPE_OLD_PALETTE1
    {
        file.seek(SeekFrom::Current(
            (chunk_header.chunk_size - CHUNKHEADER_SIZE) as i64,
        )).expect("could not seek");
        println!("ignoring old palette chunk");
    }

    // Layer chunk
    let chunk_header: ChunkHeaderC =
        deserialize_from(&mut file).expect("Could not deserialize chunk header");
    if chunk_header.chunk_type == CHUNKTYPE_LAYER {
        read_chunk_layer(&mut file).expect("Could not read layer chunk")
    }

    //let mut buf = Vec::new();
    //file.read_to_end(&mut buf);
    //println!("end {:?}", buf);
}

// -------------------------------------------------------------------------------------------------
// Palette chunk
//

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PaletteChunk {
    first_color_index: u32,
    last_color_index: u32,
    color_entries: Vec<PaletteColorEntry>,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PaletteColorEntry {
    index: u32,
    color: [u8; 4],
    name: Option<String>,
}

const PALETTE_FLAG_HAS_NAME: u16 = 1;

fn read_chunk_palette(mut file: &mut File) -> Result<(), Error> {
    let num_palette_entries: u32 = deserialize_from(&mut file)?;
    let first_color_index: u32 = deserialize_from(&mut file)?;
    let last_color_index: u32 = deserialize_from(&mut file)?;
    let _reserved: [u8; 8] = deserialize_from(&mut file)?;

    let mut chunk: PaletteChunk = Default::default();
    chunk.first_color_index = first_color_index;
    chunk.last_color_index = last_color_index;

    for index in 0..num_palette_entries {
        let entry_flags: u16 = deserialize_from(&mut file)?;
        let mut color: [u8; 4] = deserialize_from(&mut file)?;
        let name = if entry_flags & PALETTE_FLAG_HAS_NAME == PALETTE_FLAG_HAS_NAME {
            Some(deserialize_string(file)?)
        } else {
            None
        };

        let entry = PaletteColorEntry {
            index: index + chunk.first_color_index,
            color,
            name,
        };
        chunk.color_entries.push(entry);
    }

    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Layer chunk
//

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct LayerChunkC {
    pub flags: u16,
    pub layer_type: u16,
    pub layer_child_level: u16,
    pub default_layer_width_in_pixels: u16,  //ignored
    pub default_layer_height_in_pixels: u16, //ignored
    pub blend_mode: u16,
    pub opacity: u8, // TODO(JaSc): Valid only if file header flags field bit one is set
    pub _reserved: [u8; 3],
}

fn read_chunk_layer(mut file: &mut File) -> Result<(), Error> {
    let chunk: LayerChunkC = deserialize_from(&mut file)?;
    dprintln!(chunk);
    let layer_name = deserialize_string(file)?;
    Ok(())
}

fn deserialize_string(mut file: &mut File) -> Result<String, Error> {
    let string_len: u16 = deserialize_from(&mut file)?;
    let mut bytes = vec![0u8; string_len as usize];
    file.read_exact(&mut bytes)?;
    let result = std::str::from_utf8(&bytes)?.to_owned();
    Ok(result)
}
