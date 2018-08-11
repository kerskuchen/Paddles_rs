use common::*;

use std;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use bincode::serialize;
use failure;
use failure::{Error, ResultExt};
use image;
use image::DynamicImage;
use rand::prelude::*;
use rect_packer::{DensePacker, Rect};

struct SpriteData {
    filepath: String,
    rect: Rect,
}

pub fn pack_images(image_filelist: &[PathBuf], show_debug_colors: bool) -> Result<(), Error> {
    let atlas_filename = String::from("atlas.png");
    debug!("Packing atlas: {}", &atlas_filename);

    // Define input and output file-paths
    let output_dir = Path::new(DATA_DIR).join(IMAGES_DIR);
    let texture_filepath = output_dir
        .clone()
        .join(&atlas_filename)
        .with_extension("png");
    let meta_filepath = output_dir
        .clone()
        .join(&atlas_filename)
        .with_extension("tex");
    std::fs::create_dir_all(output_dir.clone())
        .context(format!("Could not create dir '{}'", output_dir.display()))?;

    // Prepare containers
    let (atlas_width, atlas_height) = (64, 64);
    let mut sprite_data = Vec::new();
    let mut packer = DensePacker::new(atlas_width, atlas_height);
    let mut atlas = DynamicImage::new_rgba8(atlas_width as u32, atlas_height as u32).to_rgba();
    if show_debug_colors {
        atlas.clear([123, 200, 250, 100]);
    }

    for image_filepath in image_filelist {
        let image_filename = filepath_to_filename_string(image_filepath)?;
        debug!("Packing image: {}", &image_filename);

        let mut source_image = image::open(image_filepath)
            .context(format!(
                "Could not open image '{}'",
                image_filepath.display()
            ))?
            .to_rgba();
        let (width, height) = source_image.dimensions();

        let dest_region = packer
            .pack(width as i32, height as i32, false)
            .ok_or(failure::err_msg(format!(
                "Not enough space to pack image: '{}'",
                image_filepath.display()
            )))?;
        let source_region = Rect {
            x: 0,
            y: 0,
            width: dest_region.width,
            height: dest_region.height,
        };
        Image::copy_region(&mut source_image, source_region, &mut atlas, dest_region);

        if show_debug_colors {
            let rand_color = [random::<u8>(), random::<u8>(), random::<u8>(), 125];
            atlas.draw_rect(dest_region, rand_color);
        }

        sprite_data.push(SpriteData {
            filepath: image_filename,
            rect: dest_region,
        });
    }
    atlas.save(texture_filepath.clone()).context(format!(
        "Could not save atlas texture to '{}'",
        texture_filepath.display()
    ))?;

    write_metadata(
        &meta_filepath,
        sprite_data,
        atlas_width as f32,
        atlas_height as f32,
    ).context(format!(
        "Could not write metadata '{}'",
        meta_filepath.display()
    ))?;

    Ok(())
}

fn write_metadata(
    meta_filepath: &Path,
    sprite_data: Vec<SpriteData>,
    image_width: f32,
    image_height: f32,
) -> Result<(), Error> {
    let mut meta_file = File::create(meta_filepath).context(format!(
        "Could not create file '{}'",
        meta_filepath.display()
    ))?;

    let sprites: Vec<_> = sprite_data
        .iter()
        .map(|data| {
            let rect = data.rect;
            let vertex_bounds = Bounds {
                left: 0.0,
                right: rect.width as f32,
                bottom: 0.0,
                top: rect.height as f32,
            };
            let uv_bounds = Bounds {
                left: rect.x as f32 / image_width,
                right: (rect.x + rect.width) as f32 / image_width,
                bottom: rect.y as f32 / image_height,
                top: (rect.y + rect.height) as f32 / image_height,
            };
            Sprite {
                vertex_bounds,
                uv_bounds,
            }
        })
        .collect();

    let sprite_mapping: Vec<_> = sprite_data
        .into_iter()
        .enumerate()
        .map(|(index, data)| (index, data.filepath))
        .collect();

    let encoded_sprite_mapping =
        serialize(&sprite_mapping).context("Could not encode sprite mapping metadata")?;
    meta_file
        .write_all(&encoded_sprite_mapping)
        .context("Could not write sprite mapping metadata")?;

    let encoded_sprites = serialize(&sprites).context("Could not encode sprite metadata")?;
    meta_file
        .write_all(&encoded_sprites)
        .context("Could not write sprite metadata")?;

    Ok(())
}
