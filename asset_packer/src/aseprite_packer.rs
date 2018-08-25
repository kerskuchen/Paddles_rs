use game_lib::{Animation, ResourcePath, Sprite, Vec2};

use common;
use common::AtlasPacker;
use common::*;

use std::collections::HashMap;
use std::process::Command;

use aseprite;
use failure::{Error, ResultExt};
use image;
use serde_json;

pub fn pack_animations_and_sprites(
    packer: &mut AtlasPacker,
) -> Result<
    (
        HashMap<ResourcePath, Sprite>,
        HashMap<ResourcePath, Animation>,
    ),
    Error,
> {
    debug!("Creating list of aseprite files");
    let image_filelist = common::collect_all_files_with_extension(common::ASSETS_DIR, "aseprite");
    trace!("Aseprite file list: {:?}", image_filelist);

    let mut sprite_map = HashMap::new();
    let mut animation_map = HashMap::new();
    for image_filepath in image_filelist {
        debug!("Packing aseprite file: '{}'", image_filepath.display());

        // get_meta_information(image_filepath.to_string());
        let pivots = get_pivots_for_file(image_filepath.to_string());
        dprintln!(pivots);

        let image = image::open(&image_filepath)
            .context(format!(
                "Could not open aseprite file '{}'",
                image_filepath.display()
            ))?
            .to_rgba();

        let offset = Vec2::zero();
        let region = packer.pack_image(image);
        let sprite = region.to_sprite(packer.atlas_size as f32, offset);

        let image_relative_filepath = image_filepath
            .strip_prefix(ASSETS_DIR)
            .context(format!(
                "Could not strip '{}' from image path {:?}",
                ASSETS_DIR,
                image_filepath.display()
            ))?
            .to_path_buf();
        let resource_path = filepath_to_string_without_extension(&image_relative_filepath)?;
        sprite_map.insert(resource_path, sprite);
    }
    Ok((sprite_map, animation_map))
}

fn get_meta_information(file_path: &str) -> Result<Vec<Vec2>, Error> {
    let result = Command::new("aseprite")
        .args(&["-b", "--list-layers", file_path])
        .output();

    let output = {
        if result.is_ok() {
            String::from_utf8_lossy(&result.unwrap().stdout).into_owned()
        } else {
            String::from("nope")
        }
    };
    println!("PATH: {:?}", output);

    let offsets = Vec::new();
    Ok(offsets)
}

fn get_pivots_for_file(file_path: &str) -> Result<Option<Vec<Vec2>>, Error> {
    let output = Command::new("aseprite")
        .args(&["-b", "--list-layers", file_path])
        .output()
        .context(format!(
            "Could not read layer information for '{}'",
            file_path
        ))?
        .stdout;

    if String::from_utf8_lossy(&output).find("pivot").is_none() {
        // This file does not have a layer with pivot points
        return Ok(None);
    }

    // NOTE: We need to split the arguments such that the "pivot" argument is passed with
    //       `arg` instead of the `args`. This lets the "pivot" argument be passed as quoted
    //       string on windows, which is what aseprite needs.
    //       For more reference see this issue:
    //       https://internals.rust-lang.org/t/std-process-on-windows-is-escaping-raw-literals-which-causes-problems-with-chaining-commands/8163
    // TODO(JaSc): We need to check if this acually works on linux as well.
    //
    let output = Command::new("aseprite")
        .args(&["-b", "--layer"])
        .arg("pivot")
        .args(&[
            "--trim",
            "--format",
            "json-array",
            file_path,
            "--sheet",
            "target/assetpacker_temp/pivot.png",
        ])
        .output()
        .context(format!(
            "Could not get offset information for '{}'",
            file_path
        ))?
        .stdout;

    let output = String::from_utf8_lossy(&output);
    let meta: aseprite::SpritesheetData =
        serde_json::from_str(&output).context("Could not deserialize commandline output")?;

    let offsets = collect_pivots_from_frames(&meta.frames);
    Ok(Some(offsets))
}

fn collect_pivots_from_frames(frames: &[aseprite::Frame]) -> Vec<Vec2> {
    let mut offsets = Vec::new();
    for frame in frames {
        let frame_index = get_frame_index_for_frame(&frame);
        let offset = get_pivot_for_frame(&frame);
        while offsets.len() < frame_index {
            offsets.push(Vec2::zero());
        }
        offsets.push(offset);
    }
    offsets
}

fn get_frame_index_for_frame(frame: &aseprite::Frame) -> usize {
    let without_suffix = frame.filename.replace(".aseprite", "");
    let parts: Vec<_> = without_suffix.split_whitespace().collect();
    let frame_index: usize = parts
        .last()
        .unwrap_or_else(|| panic!("Filename was empty for frame {:?}", frame,))
        .parse()
        .unwrap_or_else(|error| {
            panic!(
                "Could not get frame index from filename '{}' in frame {:?} : error {}",
                frame.filename, frame, error
            )
        });
    frame_index
}

fn get_pivot_for_frame(frame: &aseprite::Frame) -> Vec2 {
    if frame.frame.w != 1 || frame.frame.h != 1 {
        panic!(
            "Size of pivot point does not have 1x1 pixel size for filename '{}' in frame {:?} ",
            frame.filename, frame
        )
    }
    let pivot_x = frame.sprite_source_size.x as f32;
    let pivot_y = frame.sprite_source_size.y as f32;
    Vec2::new(pivot_x, pivot_y)
}
