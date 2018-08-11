use std;
use std::path::PathBuf;

use image;
use image::Rgba;
use rect_packer::Rect;
use walkdir::WalkDir;

use failure;
use failure::Error;

pub const ASSETS_DIR: &str = "assets";
pub const DATA_DIR: &str = "data";
pub const FONTS_DIR: &str = "fonts";
pub const IMAGES_DIR: &str = "images";

#[derive(Debug, Serialize, Deserialize)]
pub struct Sprite {
    pub vertex_bounds: Bounds,
    pub uv_bounds: Bounds,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FontHeader {
    pub num_glyphs: usize,
    pub first_code_point: u8,
}

pub fn filepath_to_filename_string(filepath: &PathBuf) -> Result<String, Error> {
    let filename = filepath
        .file_name()
        .ok_or(failure::err_msg(format!(
            "Could not retrieve filename from path {}",
            filepath.display()
        )))?
        .to_str()
        .ok_or(failure::err_msg(format!(
            "Could not convert filename to string {}",
            filepath.display()
        )))?
        .to_owned();

    Ok(filename)
}

pub fn all_files_with_extension(root_folder: &str, extension: &str) -> Vec<PathBuf> {
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

pub type Image = image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>;

pub trait ImageHelper {
    fn clear(&mut self, fill_color: [u8; 4]);
    fn draw_rect(&mut self, rect: Rect, border_color: [u8; 4]);
    fn fill_rect(&mut self, rect: Rect, fill_color: [u8; 4]);
    fn copy_region(
        source_image: &mut Image,
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
        source_image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
        source_region: Rect,
        dest_image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
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
