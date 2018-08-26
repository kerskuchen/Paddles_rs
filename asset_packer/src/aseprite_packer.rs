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
    animations: &mut HashMap<ResourcePath, Animation>,
    sprites: &mut HashMap<ResourcePath, Sprite>,
) -> Result<(), Error> {
    debug!("Creating list of aseprite files");
    let image_filelist = common::collect_all_files_with_extension(common::ASSETS_DIR, "aseprite");
    trace!("Aseprite file list: {:?}", image_filelist);

    for image_filepath in image_filelist {
        debug!("Packing aseprite file: '{}'", image_filepath.display());

        let image_relative_filepath = image_filepath
            .strip_prefix(ASSETS_DIR)
            .context(format!(
                "Could not strip '{}' from image path {:?}",
                ASSETS_DIR,
                image_filepath.display()
            ))?
            .to_path_buf();
        let resource_path = filepath_to_string_without_extension(&image_relative_filepath)?;

        let (image, meta) = get_file_image_and_meta(image_filepath.to_string()).context(format!(
            "Could not get image and meta for file '{}'",
            image_filepath.display()
        ))?;
        let region = packer.pack_image(image);
        let num_frames = meta.frames.len();
        assert!(num_frames > 0);

        let pivots = get_pivots_for_file(image_filepath.to_string(), num_frames).context(format!(
            "Could not get pivots for file '{}'",
            image_filepath.display()
        ))?;

        let mut frame_durations = Vec::new();
        let mut frames = Vec::new();

        for (frame_index, frame) in meta.frames.iter().enumerate() {
            let sprite_region = if frame.frame.w == 0 && frame.frame.h == 0 {
                // NOTE: If we encounter an empty frame we just use our default empty sprite region.
                //       If we would just use the zero rect we are given, its region would be on the
                //       wrong atlas index and possibly land on a nonempty sprite.
                packer.default_empty_region()
            } else {
                let sub_rect_relative = Rect {
                    x: frame.frame.x as i32,
                    y: frame.frame.y as i32,
                    width: frame.frame.w as i32,
                    height: frame.frame.h as i32,
                };
                region.sub_region_relative(sub_rect_relative)
            };

            let offset_x = frame.sprite_source_size.x as f32 - pivots[frame_index].x;
            let offset_y = frame.sprite_source_size.y as f32 - pivots[frame_index].y;
            let offset = Vec2::new(offset_x, offset_y);

            let duration = frame.duration as f32;
            let sprite = sprite_region.to_sprite(packer.atlas_size, offset);

            frame_durations.push(duration);
            frames.push(sprite);
        }

        if num_frames == 1 {
            sprites.insert(resource_path, frames[0]);
        } else {
            animations.insert(
                resource_path,
                Animation {
                    frame_durations,
                    frames,
                },
            );
        }
    }

    Ok(())
}

fn get_file_image_and_meta(file_path: &str) -> Result<(Image, aseprite::SpritesheetData), Error> {
    const OUTPUT_FILEPATH: &str = "target/assetpacker_temp/output.png";
    // NOTE: We need to split the arguments such that the "pivot" argument is passed with
    //       `arg` instead of the `args`. This lets the "pivot" argument be passed as quoted
    //       string on windows, which is what aseprite needs.
    //       For more reference see this issue:
    //       https://internals.rust-lang.org/t/std-process-on-windows-is-escaping-raw-literals-which-causes-problems-with-chaining-commands/8163
    // TODO(JaSc): We need to check if this acually works on linux as well.
    //
    let output = Command::new("aseprite")
        .args(&["-b", "--ignore-layer"])
        .arg("pivot")
        .args(&[
            "--trim",
            "--format",
            "json-array",
            file_path,
            "--sheet",
            OUTPUT_FILEPATH,
        ])
        .output()
        .context("Could not get valid command line output while getting offset information")?
        .stdout;

    let output = String::from_utf8_lossy(&output);
    let meta: aseprite::SpritesheetData = serde_json::from_str(&output)
        .context("Could not deserialize commandline output while getting offset information")?;

    let image = image::open(OUTPUT_FILEPATH)
        .context("Could not open output image")?
        .to_rgba();

    Ok((image, meta))
}

fn get_pivots_for_file(file_path: &str, num_frames: usize) -> Result<Vec<Vec2>, Error> {
    let mut pivots = vec![Vec2::zero(); num_frames];

    if list_layers_of_file(file_path)?.find("pivot").is_none() {
        // This file does not have a layer with pivot points
        return Ok(pivots);
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

    collect_pivots_from_frames(&meta.frames, &mut pivots);

    Ok(pivots)
}

fn list_layers_of_file(file_path: &str) -> Result<String, Error> {
    let output = Command::new("aseprite")
        .args(&["-b", "--list-layers", file_path])
        .output()
        .context("Could not read layer information")?
        .stdout;
    Ok(String::from_utf8_lossy(&output).into_owned())
}

fn collect_pivots_from_frames(frames: &[aseprite::Frame], pivots: &mut Vec<Vec2>) {
    for frame in frames {
        let frame_index = get_frame_index_for_frame(&frame);
        let pivot = get_pivot_for_frame(&frame);
        pivots[frame_index] = pivot;
    }
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
