use game_lib::{ResourcePath, Sprite, Vec2};

use crate::common;
use crate::common::AtlasPacker;
use crate::common::*;

use std::collections::HashMap;

use failure::{Error, ResultExt};
use image;

pub fn pack_sprites(
    packer: &mut AtlasPacker,
    sprites: &mut HashMap<ResourcePath, Sprite>,
) -> Result<(), Error> {
    debug!("Creating list of images");
    let image_filelist = common::collect_all_files_with_extension(common::ASSETS_DIR, "png");
    trace!("Image list: {:?}", image_filelist);

    for image_filepath in image_filelist {
        debug!("Packing image: '{}'", image_filepath.display());

        let image = image::open(&image_filepath)
            .context(format!(
                "Could not open image '{}'",
                image_filepath.display()
            ))?
            .to_rgba();

        let offset = Vec2::zero();
        let region = packer.pack_image(image);
        let sprite = region.to_sprite(packer.atlas_size, offset);

        let image_relative_filepath = image_filepath
            .strip_prefix(ASSETS_DIR)
            .context(format!(
                "Could not strip '{}' from image path {:?}",
                ASSETS_DIR,
                image_filepath.display()
            ))?
            .to_path_buf();
        let resource_path = filepath_to_string_without_extension(&image_relative_filepath)?;
        sprites.insert(resource_path, sprite);
    }
    Ok(())
}
