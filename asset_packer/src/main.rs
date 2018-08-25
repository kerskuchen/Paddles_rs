/*
TODO(JaSc):
  - Create proper font metadata with max-glyph-height and glyph metadata with
    - rect in bitmap
    - x,y offsets 
    - x advance
  - Create proper spritesheet metadata with
    - rect in bitmap
    - frame duration
    - x,y offsets from pivot-point information
  - Embed all spritesheets and font-textures into one atlas with corrected metadata
  - Create actual vertex/uv rects metadata from the whole atlas. Maybe group animation-sprites and 
    font-sprites into Vectors which can be accessed via hashmap.
*/

#[macro_use]
extern crate game_lib;
use game_lib::AtlasMeta;

extern crate image;
extern crate rand;
extern crate rect_packer;
extern crate rusttype;

extern crate inflate;

#[macro_use]
extern crate log;
extern crate fern;

#[macro_use]
extern crate serde_derive;
extern crate aseprite;
extern crate bincode;
extern crate ron;
extern crate serde;
extern crate serde_json;

extern crate failure;
use failure::{Error, ResultExt};

extern crate walkdir;

pub mod aseprite_packer;
pub mod common;
pub mod font_packer;
pub mod image_packer;

const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

use bincode::serialize;
use common::AtlasPacker;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Error> {
    // Initialize logger
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}-{}: {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LOG_LEVEL)
        .chain(std::io::stdout())
        .apply()
        .context("Could not initialize logger")?;

    let mut packer = AtlasPacker::new(32);

    debug!("Packing fonts");
    let font_map = font_packer::pack_fonts(&mut packer)?;
    info!("Successfully packed fonts");

    debug!("Packing images and animations");
    let (animations_map, sprite_map) = aseprite_packer::pack_animations_and_sprites(&mut packer)?;
    info!("Successfully packed images and animations");

    debug!("Packing png images");
    let sprite_map = image_packer::pack_sprites(&mut packer)?;
    info!("Successfully packed png images");

    debug!("Saving atlas textures");
    let atlases = packer.into_atlas_textures();
    for (atlas_index, atlas) in atlases.iter().enumerate() {
        atlas
            .save(format!("data/atlas_{}.png", atlas_index))
            .context(format!("Could not save atlas{}", atlas_index))?;
    }
    info!("Successfully saved atlas textures");

    debug!("Saving atlas metadata");
    let atlas_meta = AtlasMeta {
        num_atlas_textures: atlases.len(),
        fonts: font_map,
        animations: HashMap::new(),
        sprites: sprite_map,
    };

    let meta_filepath = "data/atlas.tex";
    let mut meta_file =
        File::create(meta_filepath).context(format!("Could not create file '{}'", meta_filepath))?;
    let encoded_sprites = serialize(&atlas_meta).context("Could not encode atlas metadata")?;
    meta_file
        .write_all(&encoded_sprites)
        .context("Could not write atlas metadata")?;
    info!("Successfully saved atlas metadata");

    Ok(())
}
