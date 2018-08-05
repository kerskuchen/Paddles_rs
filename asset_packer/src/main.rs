extern crate image;
extern crate rusttype;

use image::{DynamicImage, Rgba};
use rusttype::{point, Font, Scale};
use std::fs::File;
use std::io::Read;

fn main() {
    let font_data = {
        let mut font_data = Vec::new();
        File::open("assets/04B_03__.TTF")
            .unwrap_or_else(|error| panic!("Error opening font file: {}", error))
            .read_to_end(&mut font_data)
            .unwrap_or_else(|error| panic!("Error reading font file: {}", error));
        font_data
    };

    let font = Font::from_bytes(&font_data)
        .unwrap_or_else(|error| panic!("Error constructing font: {}", error));

    let scale = Scale::uniform(32.0);
    let text = "Hello klains lalalala!";
    let color = (0, 200, 0);
    let metrics = font.v_metrics(scale);

    let glyphs: Vec<_> = font
        .layout(text, scale, point(20.0, 20.0 + metrics.ascent))
        .collect();

    let text_height = (metrics.ascent - metrics.descent).ceil() as u32;
    let text_width = {
        let left = glyphs
            .first()
            .map(|glyph| glyph.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let right = glyphs
            .last()
            .map(|glyph| glyph.pixel_bounding_box().unwrap().max.x)
            .unwrap();
        (right - left) as u32
    };

    let mut image = DynamicImage::new_rgba8(text_width + 40, text_height + 40).to_rgba();
    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                image.put_pixel(
                    x + bounding_box.min.x as u32,
                    y + bounding_box.min.y as u32,
                    Rgba {
                        data: [color.0, color.1, color.2, (v * 255.0) as u8],
                    },
                )
            });
        }
    }

    std::fs::create_dir_all("data/fonts")
        .unwrap_or_else(|error| panic!("Cannot create dir 'data': {}", error));
    image
        .save("data/fonts/test.png")
        .unwrap_or_else(|error| panic!("Error saving image: {}", error));

    println!("Packed font successfully");
}
