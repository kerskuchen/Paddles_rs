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

    const COLOR_GLYPH: [u8; 4] = [255, 255, 255, 255];
    const COLOR_BORDER: [u8; 4] = [0, 0, 0, 255];

    let mut image = DynamicImage::new_rgba8(text_width + 40, text_height + 40).to_rgba();
    for glyph in &glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                if v > 0.5 {
                    image.put_pixel(
                        x + bounding_box.min.x as u32,
                        y + bounding_box.min.y as u32,
                        Rgba { data: COLOR_GLYPH },
                    )
                }
            });
        }
    }

    // Create a border around every glyph in the image
    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel_val = image.get_pixel(x, y).clone();
            if pixel_val.data == COLOR_GLYPH {
                // We landed on a glyph's pixel. We need to paint a border in our neighbouring
                // pixels that are not themselves part of a glyph
                let pairs = vec![(-1, 0), (1, 0), (0, -1), (0, 1), (1, 1)];
                for pair in pairs {
                    let neighbour_x = (x as i32 + pair.0) as u32;
                    let neighbour_y = (y as i32 + pair.1) as u32;
                    let neighbour_pixel_val = image.get_pixel(neighbour_x, neighbour_y).clone();

                    if neighbour_pixel_val.data != COLOR_GLYPH {
                        image.put_pixel(neighbour_x, neighbour_y, Rgba { data: COLOR_BORDER })
                    }
                }
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
