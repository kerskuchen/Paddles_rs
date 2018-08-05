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

    let scale = Scale::uniform(8.0);
    let font = Font::from_bytes(&font_data)
        .unwrap_or_else(|error| panic!("Error constructing font: {}", error));

    let metrics = font.v_metrics(scale);
    let text = "Test gggg";

    let glyphs: Vec<_> = font
        .layout(text, scale, point(20.0, 20.0 + metrics.ascent))
        .collect();

    let text_height = (metrics.ascent - metrics.descent).ceil() as u32;
    println!("{}", text_height);
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
    for glyph in &glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                if v > 0.5 {
                    image.put_pixel(
                        x + bounding_box.min.x as u32,
                        y + bounding_box.min.y as u32,
                        Rgba {
                            data: [255, 255, 255, 255],
                        },
                    )
                }
            });
        }
    }

    for y in 0..image.height() {
        for x in 0..image.width() {
            if (y as i32) - 1 >= 0
                && image.get_pixel(x, y - 1)[2] != 0
                && image.get_pixel(x, y)[3] == 0
            {
                image.put_pixel(
                    x,
                    y,
                    Rgba {
                        data: [0, 0, 0, 255],
                    },
                )
            }
            if (x as i32) - 1 >= 0
                && image.get_pixel(x - 1, y)[2] != 0
                && image.get_pixel(x, y)[3] == 0
            {
                image.put_pixel(
                    x,
                    y,
                    Rgba {
                        data: [0, 0, 0, 255],
                    },
                )
            }
            if x + 1 < image.width()
                && image.get_pixel(x + 1, y)[2] != 0
                && image.get_pixel(x, y)[3] == 0
            {
                image.put_pixel(
                    x,
                    y,
                    Rgba {
                        data: [0, 0, 0, 255],
                    },
                )
            }
            if y + 1 < image.height()
                && image.get_pixel(x, y + 1)[2] != 0
                && image.get_pixel(x, y)[3] == 0
            {
                image.put_pixel(
                    x,
                    y,
                    Rgba {
                        data: [0, 0, 0, 255],
                    },
                )
            }
            if (x as i32) - 1 >= 0
                && (y as i32) - 1 >= 0
                && image.get_pixel(x - 1, y - 1)[2] != 0
                && image.get_pixel(x, y)[3] == 0
            {
                image.put_pixel(
                    x,
                    y,
                    Rgba {
                        data: [0, 0, 0, 255],
                    },
                )
            }
        }
    }

    std::fs::create_dir_all("data/fonts")
        .unwrap_or_else(|error| panic!("Cannot create dir 'data': {}", error));
    image
        .save("data/fonts/test.png")
        .unwrap_or_else(|error| panic!("Error saving image: {}", error));

    println!("Packed font successfully");
}
