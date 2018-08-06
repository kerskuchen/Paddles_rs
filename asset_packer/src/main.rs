extern crate game_lib;
extern crate image;
extern crate rand;
extern crate rect_packer;
extern crate rusttype;

use image::{DynamicImage, Rgba};
use rand::prelude::*;
use rect_packer::DensePacker;
use rusttype::{point, Font, Scale};
use std::fs::File;
use std::io::Read;

#[derive(Debug, Clone, Copy)]
struct GlyphRect {
    code_point: char,
    outer_rect: PixelRect,
    inner_rect: PixelRect,
}
impl GlyphRect {
    fn new(code_point: char) -> GlyphRect {
        GlyphRect {
            code_point,
            outer_rect: PixelRect::zero(),
            inner_rect: PixelRect::zero(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PixelRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl PixelRect {
    fn zero() -> PixelRect {
        PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

fn main() {
    let font_data = {
        let mut font_data = Vec::new();
        File::open("assets/04B_03__.TTF")
            .unwrap_or_else(|error| panic!("Error opening font file: {}", error))
            .read_to_end(&mut font_data)
            .unwrap_or_else(|error| panic!("Error reading font file: {}", error));
        font_data
    };

    let font_height = 8.0;

    let scale = Scale::uniform(font_height);
    let font = Font::from_bytes(&font_data)
        .unwrap_or_else(|error| panic!("Error constructing font: {}", error));

    let code_points: Vec<char> = (32..=126).map(|byte| (byte as u8) as char).collect();

    let metrics = font.v_metrics(scale);
    let text_height = (metrics.ascent - metrics.descent).ceil() as i32;
    let descent = metrics.descent.ceil() as i32;

    let mut image = DynamicImage::new_rgba8(64, 64).to_rgba();
    let mut packer = DensePacker::new(64, 64);

    const COLOR_GLYPH: [u8; 4] = [255, 255, 255, 255];
    const COLOR_BORDER: [u8; 4] = [0, 0, 0, 255];

    let show_debug_colors = true;
    for code_point in code_points {
        let mut glyph_rect = GlyphRect::new(code_point);
        let glyph = font
            .glyph(code_point)
            .scaled(scale)
            .positioned(point(0.0, font_height));

        let glyph_metrics = glyph.unpositioned().h_metrics();
        glyph_rect.outer_rect.width = glyph_metrics.advance_width.ceil() as i32;
        glyph_rect.outer_rect.height = text_height;

        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph_rect.inner_rect.width = bounding_box.width();
            glyph_rect.inner_rect.height = bounding_box.height();
            glyph_rect.inner_rect.y = bounding_box.min.y + descent;
            glyph_rect.inner_rect.x =
                bounding_box.min.x + glyph_metrics.left_side_bearing.ceil() as i32;
            if glyph_rect.inner_rect.x < 0 {
                // NOTE: If we ever reach here, it means we need to overhaul our font rendering to
                //       incorporate negative horizontal offsets. It would mean that the left-most
                //       pixel of the glyph is outside of the left outer_rect boundary
                panic!(
                    "The x offset of code-point {} was less than zero ({})",
                    code_point, bounding_box.min.x
                );
            }
        } else {
            glyph_rect.inner_rect.width = glyph_rect.outer_rect.width;
            glyph_rect.inner_rect.height = glyph_rect.outer_rect.height;
        }

        let packed_rect = packer
            .pack(
                glyph_rect.outer_rect.width,
                glyph_rect.outer_rect.height,
                false,
            )
            .unwrap_or_else(|| {
                panic!(
                    "Not enough space to pack glyph for code_point: {}",
                    code_point
                )
            });

        glyph_rect.outer_rect.x = packed_rect.x;
        glyph_rect.outer_rect.y = packed_rect.y;

        // Debug coloring
        if show_debug_colors {
            let rand_color = [
                random::<u8>(),
                random::<u8>(),
                random::<u8>(),
                random::<u8>(),
            ];
            for y in glyph_rect.outer_rect.y..glyph_rect.outer_rect.y + glyph_rect.outer_rect.height
            {
                for x in
                    glyph_rect.outer_rect.x..glyph_rect.outer_rect.x + glyph_rect.outer_rect.width
                {
                    image.put_pixel(x as u32, y as u32, Rgba { data: rand_color })
                }
            }
        }

        glyph.draw(|x, y, v| {
            if v > 0.5 {
                image.put_pixel(
                    x + (glyph_rect.outer_rect.x + glyph_rect.inner_rect.x) as u32,
                    y + (glyph_rect.outer_rect.y + glyph_rect.inner_rect.y) as u32,
                    Rgba { data: COLOR_GLYPH },
                )
            }
        });
        println!("{} : {}", code_point as u8, code_point);
        println!("{:?}", glyph.unpositioned().h_metrics());
        println!("{:?}", glyph.pixel_bounding_box());
        println!("{:?}", glyph_rect);
    }

    //draw_border(&mut image, COLOR_GLYPH, COLOR_BORDER);

    std::fs::create_dir_all("data/fonts")
        .unwrap_or_else(|error| panic!("Cannot create dir 'data': {}", error));
    image
        .save("data/fonts/test.png")
        .unwrap_or_else(|error| panic!("Error saving image: {}", error));

    println!("Packed font successfully");
}

fn draw_border(
    image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
    color_glyph: [u8; 4],
    color_border: [u8; 4],
) {
    // Create a border around every glyph in the image
    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel_val = image.get_pixel(x, y).clone();
            if pixel_val.data == color_glyph {
                // We landed on a glyph's pixel. We need to paint a border in our neighbouring
                // pixels that are not themselves part of a glyph
                let pairs = vec![(-1, 0), (1, 0), (0, -1), (0, 1), (1, 1)];
                for pair in pairs {
                    let neighbour_x = (x as i32 + pair.0) as u32;
                    let neighbour_y = (y as i32 + pair.1) as u32;
                    let neighbour_pixel_val = image.get_pixel(neighbour_x, neighbour_y).clone();

                    if neighbour_pixel_val.data != color_glyph {
                        image.put_pixel(neighbour_x, neighbour_y, Rgba { data: color_border })
                    }
                }
            }
        }
    }
}
