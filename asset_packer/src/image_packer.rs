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

pub fn pack_images(image_filelist: &[PathBuf], show_debug_colors: bool) -> Result<(), Error> {
    let atlas_filename = String::from("atlas.png");
    debug!("Packing atlas: {}", &atlas_filename);

    let (atlas_width, atlas_height) = (64, 64);
    let mut packer = DensePacker::new(atlas_width, atlas_height);
    let mut atlas = DynamicImage::new_rgba8(atlas_width as u32, atlas_height as u32).to_rgba();

    if show_debug_colors {
        atlas.clear([123, 200, 250, 100]);
    }

    for image_filepath in image_filelist {
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
    }

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

    atlas.save(texture_filepath.clone()).context(format!(
        "Could not save atlas texture to '{}'",
        texture_filepath.display()
    ))?;

    Ok(())
}
