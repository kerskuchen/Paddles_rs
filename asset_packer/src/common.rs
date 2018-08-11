use std;
use std::path::PathBuf;

use image;
use image::Rgba;
use rect_packer::Rect;
use walkdir::WalkDir;

pub const ASSETS_DIR: &str = "assets";
pub const DATA_DIR: &str = "data";
pub const FONTS_DIR: &str = "fonts";

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
    pub last_code_point: u8,
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

pub fn clear_image(
    image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
    fill_color: [u8; 4],
) {
    let rect = Rect::new(0, 0, image.width() as i32, image.height() as i32);
    fill_rect(image, rect, fill_color);
}

pub fn fill_rect(
    image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
    rect: Rect,
    fill_color: [u8; 4],
) {
    assert!(rect.x >= 0);
    assert!(rect.y >= 0);
    assert!(rect.x + rect.width <= image.width() as i32);
    assert!(rect.y + rect.height <= image.height() as i32);

    for y in rect.y..(rect.y + rect.height) {
        for x in rect.x..(rect.x + rect.width) {
            image.put_pixel(x as u32, y as u32, Rgba { data: fill_color })
        }
    }
}
