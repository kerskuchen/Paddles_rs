use std;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use image;
use image::DynamicImage;
use image::Rgba;
use rand::prelude::*;
use rect_packer::{DensePacker, Rect};
use walkdir::WalkDir;

use failure;
use failure::Error;

use game_lib;
use game_lib::{Sprite, Vec2};

//==================================================================================================
// Paths
//==================================================================================================
//
pub const ASSETS_DIR: &str = "assets";
pub const DATA_DIR: &str = "data";
pub const FONTS_DIR: &str = "fonts";
pub const FONTS_PATH: &str = "assets/fonts/";
pub const IMAGES_DIR: &str = "images";

pub trait PathHelper {
    fn to_string_owned(&self) -> String;
    fn to_string(&self) -> &str;
}

impl PathHelper for Path {
    fn to_string_owned(&self) -> String {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to String {:?}", self))
            .to_owned()
    }

    fn to_string(&self) -> &str {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to &str {:?}", self))
    }
}

impl PathHelper for OsStr {
    fn to_string_owned(&self) -> String {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to String {:?}", self))
            .to_owned()
    }

    fn to_string(&self) -> &str {
        self.to_str()
            .unwrap_or_else(|| panic!("Could not convert path to &str {:?}", self))
    }
}

pub fn filepath_to_filename_string(filepath: &Path) -> Result<String, Error> {
    Ok(filepath
        .file_name()
        .ok_or_else(|| {
            failure::err_msg(format!(
                "Could not retrieve filename from path {}",
                filepath.display()
            ))
        })?
        .to_string_owned())
}

pub fn filepath_to_filename_string_without_extension(filepath: &Path) -> Result<String, Error> {
    Ok(filepath_to_filename_string(&filepath.with_extension(""))?)
}

pub fn filepath_to_string_without_extension(filepath: &Path) -> Result<String, Error> {
    Ok(filepath
        .with_extension("")
        .as_os_str()
        .to_string_owned()
        .replace("\\", "/"))
}

pub fn collect_all_files_with_extension(root_folder: &str, extension: &str) -> Vec<PathBuf> {
    WalkDir::new(root_folder)
        .into_iter()
        .filter_map(|maybe_entry| maybe_entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| {
            path.extension().is_some()
                && path
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap()
                    .to_lowercase() == extension.to_lowercase()
        })
        .collect()
}

//==================================================================================================
// Image
//==================================================================================================
//
pub type Image = image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>;

pub trait ImageHelper {
    fn clear(&mut self, fill_color: [u8; 4]);
    fn draw_rect(&mut self, rect: Rect, border_color: [u8; 4]);
    fn fill_rect(&mut self, rect: Rect, fill_color: [u8; 4]);
    fn copy_region(
        source_image: &Image,
        source_region: Rect,
        dest_image: &mut Image,
        dest_region: Rect,
    );
}

impl ImageHelper for Image {
    fn clear(&mut self, fill_color: [u8; 4]) {
        let rect = Rect::new(0, 0, self.width() as i32, self.height() as i32);
        self.fill_rect(rect, fill_color);
    }

    fn draw_rect(&mut self, rect: Rect, border_color: [u8; 4]) {
        assert!(rect.x >= 0);
        assert!(rect.y >= 0);
        assert!(rect.x + rect.width <= self.width() as i32);
        assert!(rect.y + rect.height <= self.height() as i32);

        for y in rect.y..(rect.y + rect.height) {
            for x in rect.x..(rect.x + rect.width) {
                if x == rect.x
                    || y == rect.y
                    || x == (rect.x + rect.width - 1)
                    || y == (rect.y + rect.height - 1)
                {
                    self.put_pixel(x as u32, y as u32, Rgba { data: border_color })
                }
            }
        }
    }

    fn fill_rect(&mut self, rect: Rect, fill_color: [u8; 4]) {
        assert!(rect.x >= 0);
        assert!(rect.y >= 0);
        assert!(rect.x + rect.width <= self.width() as i32);
        assert!(rect.y + rect.height <= self.height() as i32);

        for y in rect.y..(rect.y + rect.height) {
            for x in rect.x..(rect.x + rect.width) {
                self.put_pixel(x as u32, y as u32, Rgba { data: fill_color })
            }
        }
    }

    fn copy_region(
        source_image: &Image,
        source_region: Rect,
        dest_image: &mut Image,
        dest_region: Rect,
    ) {
        assert!(source_region.width == dest_region.width);
        assert!(source_region.height == dest_region.height);

        assert!(source_region.x >= 0);
        assert!(source_region.y >= 0);
        assert!(source_region.x + source_region.width <= source_image.width() as i32);
        assert!(source_region.y + source_region.height <= source_image.height() as i32);

        assert!(dest_region.x >= 0);
        assert!(dest_region.y >= 0);
        assert!(dest_region.x + dest_region.width <= dest_image.width() as i32);
        assert!(dest_region.y + dest_region.height <= dest_image.height() as i32);

        for y in 0..source_region.height {
            for x in 0..source_region.width {
                let source_color = source_image
                    .get_pixel((x + source_region.x) as u32, (y + source_region.y) as u32)
                    .data;

                dest_image.put_pixel(
                    (x + dest_region.x) as u32,
                    (y + dest_region.y) as u32,
                    Rgba { data: source_color },
                )
            }
        }
    }
}

//==================================================================================================
// AtlasPacker
//==================================================================================================
//
const DEBUG_DRAW_RECTS_AROUND_SPRITES: bool = false;

#[derive(Copy, Clone)]
pub struct AtlasRegion {
    rect: Rect,
    atlas_index: u32,
}

impl AtlasRegion {
    pub fn to_sprite(&self, atlas_size: f32, offset: Vec2) -> Sprite {
        let rect = self.rect;
        let vertex_bounds = game_lib::Rect {
            left: offset.x,
            right: offset.x + rect.width as f32,
            top: offset.y,
            bottom: offset.y + rect.height as f32,
        };
        let uv_bounds = game_lib::Rect {
            left: rect.x as f32 / atlas_size,
            right: (rect.x + rect.width) as f32 / atlas_size,
            top: rect.y as f32 / atlas_size,
            bottom: (rect.y + rect.height) as f32 / atlas_size,
        };
        Sprite {
            vertex_bounds,
            uv_bounds,
            atlas_index: self.atlas_index,
        }
    }
}

pub struct AtlasPacker {
    rect_packer: AtlasRectPacker,
    texture_writer: AtlasTextureWriter,
    default_empty_region: AtlasRegion,
    pub atlas_size: i32,
}

impl AtlasPacker {
    pub fn new(atlas_size: i32) -> AtlasPacker {
        let mut rect_packer = AtlasRectPacker::new(atlas_size);
        let default_empty_region = rect_packer.pack_image(1, 1);

        AtlasPacker {
            rect_packer,
            texture_writer: AtlasTextureWriter::new(atlas_size as u32),
            default_empty_region,
            atlas_size,
        }
    }

    pub fn default_empty_region(&self) -> AtlasRegion {
        self.default_empty_region
    }

    pub fn pack_image(&mut self, image: Image) -> AtlasRegion {
        let region = self
            .rect_packer
            .pack_image(image.width() as i32, image.height() as i32);
        self.texture_writer.write_image(image, &region);
        region
    }

    pub fn into_atlas_textures(self) -> Vec<Image> {
        self.texture_writer.into_atlas_textures()
    }
}

struct AtlasRectPacker {
    atlas_size: i32,
    atlas_packers: Vec<DensePacker>,
}

impl AtlasRectPacker {
    fn new(atlas_size: i32) -> AtlasRectPacker {
        AtlasRectPacker {
            atlas_size,
            atlas_packers: vec![DensePacker::new(atlas_size, atlas_size)],
        }
    }

    fn pack_image(&mut self, image_width: i32, image_height: i32) -> AtlasRegion {
        for (atlas_index, mut packer) in self.atlas_packers.iter_mut().enumerate() {
            if let Some(rect) = packer.pack(image_width, image_height, false) {
                return AtlasRegion {
                    rect,
                    atlas_index: atlas_index as u32,
                };
            }
        }

        // At this point our image did not fit in any of the existing atlases,
        // so we create a new atlas.
        let mut atlas = DensePacker::new(self.atlas_size, self.atlas_size);
        let rect = atlas
            .pack(image_width, image_height, false)
            .unwrap_or_else(|| {
                panic!(
                    "Could not pack image with dimensions {}x{} into atlas with dimensions {}x{}",
                    image_width, image_height, self.atlas_size, self.atlas_size
                )
            });
        let atlas_index = self.atlas_packers.len() as u32;
        self.atlas_packers.push(atlas);

        AtlasRegion { rect, atlas_index }
    }
}

struct AtlasTextureWriter {
    atlas_size: u32,
    atlas_textures: Vec<Image>,
}

impl AtlasTextureWriter {
    fn new(atlas_size: u32) -> AtlasTextureWriter {
        AtlasTextureWriter {
            atlas_size,
            atlas_textures: Vec::new(),
        }
    }

    fn write_image(&mut self, image: Image, region: &AtlasRegion) {
        self.add_more_atlases_if_necessary(region.atlas_index as usize);

        let dest_image = &mut self.atlas_textures[region.atlas_index as usize];
        let dest_rect = region.rect;

        let source_image = image;
        let source_rect = Rect {
            x: 0,
            y: 0,
            width: dest_rect.width,
            height: dest_rect.height,
        };

        Image::copy_region(&source_image, source_rect, dest_image, dest_rect);

        if DEBUG_DRAW_RECTS_AROUND_SPRITES {
            let rand_color = [random::<u8>(), random::<u8>(), random::<u8>(), 125];
            dest_image.draw_rect(dest_rect, rand_color);
        }
    }

    fn into_atlas_textures(self) -> Vec<Image> {
        self.atlas_textures
    }

    fn add_more_atlases_if_necessary(&mut self, atlas_index: usize) {
        while atlas_index >= self.atlas_textures.len() {
            let atlas = DynamicImage::new_rgba8(self.atlas_size, self.atlas_size).to_rgba();
            self.atlas_textures.push(atlas)
        }
    }
}
