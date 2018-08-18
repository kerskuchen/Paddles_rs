extern crate game_lib;

extern crate image;
extern crate rand;
extern crate rect_packer;
extern crate rusttype;

#[macro_use]
extern crate log;
extern crate fern;

extern crate bincode;
extern crate serde;

extern crate failure;
use failure::{Error, ResultExt};

extern crate walkdir;

pub mod common;
pub mod font_packer;
pub mod image_packer;

const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

fn main() -> Result<(), Error> {
    // Initializing logger
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

    debug!("Packing fonts");
    let font_height = 8.0;
    let show_debug_colors = false;
    let do_draw_border = true;
    let font_filelist = common::collect_all_files_with_extension(common::ASSETS_DIR, "ttf");

    trace!("Font list: {:?}", font_filelist);
    for font_filepath in font_filelist {
        font_packer::pack_font(
            &font_filepath,
            font_height,
            do_draw_border,
            show_debug_colors,
        ).context(format!("Could not pack font {}", font_filepath.display()))?;
    }
    info!("Successfully packed fonts");

    debug!("Packing images");
    let image_filelist = common::collect_all_files_with_extension(common::ASSETS_DIR, "png");
    trace!("Image list: {:?}", image_filelist);

    image_packer::pack_images(&image_filelist, show_debug_colors)
        .context("Could not pack atlases")?;

    info!("Successfully packed images");

    Ok(())
}
