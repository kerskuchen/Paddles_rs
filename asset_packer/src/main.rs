//#[macro_use]
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

const DEBUG_WRITE_HUMAN_READABLE_ATLAS_META: bool = true;
const ATLAS_TEXTURE_SIZE: u32 = 64;

const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

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

    let mut packer = AtlasPacker::new(ATLAS_TEXTURE_SIZE);
    let mut animations = HashMap::new();
    let mut sprites = HashMap::new();
    let mut fonts = HashMap::new();

    debug!("Packing fonts");
    font_packer::pack_fonts(&mut packer, &mut fonts)?;
    info!("Successfully packed fonts");

    debug!("Packing images and animations");
    aseprite_packer::pack_animations_and_sprites(&mut packer, &mut animations, &mut sprites)?;
    info!("Successfully packed images and animations");

    debug!("Packing png images");
    image_packer::pack_sprites(&mut packer, &mut sprites)?;
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
        fonts,
        animations,
        sprites,
    };
    let meta_filepath = "data/atlas.tex";
    let mut meta_file =
        File::create(meta_filepath).context(format!("Could not create file '{}'", meta_filepath))?;
    let encoded_sprites = serialize(&atlas_meta).context("Could not encode atlas metadata")?;
    meta_file
        .write_all(&encoded_sprites)
        .context("Could not write atlas metadata")?;
    info!("Successfully saved atlas metadata");

    if DEBUG_WRITE_HUMAN_READABLE_ATLAS_META {
        let debug_output = format!("{:#?}", atlas_meta);
        std::fs::write("data/atlas_debug.txt", &debug_output)?;
    }

    Ok(())
}
