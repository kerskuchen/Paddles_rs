extern crate game_lib;
extern crate image;
extern crate rand;
extern crate rect_packer;
extern crate rusttype;

use image::{DynamicImage, Rgba};
use rand::prelude::*;
use rect_packer::{DensePacker, Rect};
use rusttype::{point, Font, PositionedGlyph, Scale};
use std::fs::File;
use std::io::Read;

fn main() {
    // Configuration
    let font_height = 8.0;
    let do_draw_border = true;
    let show_debug_colors = true;
    let padding = if do_draw_border { 1 } else { 0 };
    const COLOR_GLYPH: [u8; 4] = [255, 255, 255, 255];
    const COLOR_BORDER: [u8; 4] = [0, 0, 0, 255];

    // Creating font
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

    // Packing glyphs
    let code_points: Vec<char> = (32..=126).map(|byte| (byte as u8) as char).collect();
    let mut packer = DensePacker::new(96, 70);
    let glyph_data: Vec<_> = code_points
        .iter()
        .map(|&code_point| pack_glyph(&font, &mut packer, code_point, font_height, padding))
        .collect();

    // Creating image
    let (image_width, image_height) = packer.size();
    let mut image = DynamicImage::new_rgba8(image_width as u32, image_height as u32).to_rgba();
    if show_debug_colors {
        clear_image(&mut image, [123, 200, 250, 100]);
    }

    // Draw glyphs
    for (code_point, glyph, inner_rect, outer_rect) in glyph_data {
        if show_debug_colors {
            // Visualize outer rect
            let rand_color = [random::<u8>(), random::<u8>(), random::<u8>(), 150];
            fill_rect(&mut image, outer_rect, rand_color);

            // Visualize inner rect
            let rand_color = [random::<u8>(), random::<u8>(), random::<u8>(), 255];
            let inner_rect_with_padding = Rect {
                x: inner_rect.x + outer_rect.x - padding,
                y: inner_rect.y + outer_rect.y - padding,
                width: inner_rect.width + padding,
                height: inner_rect.height + padding,
            };
            fill_rect(&mut image, inner_rect_with_padding, rand_color);
        }

        // Draw actual glyphs
        glyph.draw(|x, y, v| {
            if v > 0.5 {
                image.put_pixel(
                    x + (outer_rect.x + inner_rect.x) as u32,
                    y + (outer_rect.y + inner_rect.y) as u32,
                    Rgba { data: COLOR_GLYPH },
                )
            }
        });
    }
    if do_draw_border {
        draw_border(&mut image, COLOR_GLYPH, COLOR_BORDER);
    }

    // Write out image
    std::fs::create_dir_all("data/fonts")
        .unwrap_or_else(|error| panic!("Cannot create dir 'data': {}", error));
    image
        .save("data/fonts/test.png")
        .unwrap_or_else(|error| panic!("Error saving image: {}", error));
    println!("Packed font successfully");
}

fn pack_glyph<'a>(
    font: &'a Font,
    packer: &mut DensePacker,
    code_point: char,
    font_height: f32,
    padding: i32,
) -> (char, PositionedGlyph<'a>, Rect, Rect) {
    // Font metrics
    let scale = Scale::uniform(font_height);
    let metrics = font.v_metrics(scale);
    let text_height = (metrics.ascent - metrics.descent).ceil() as i32;
    let descent = metrics.descent.ceil() as i32;

    // Glyph metrics
    let glyph = font
        .glyph(code_point)
        .scaled(scale)
        .positioned(point(0.0, font_height));
    let glyph_metrics = glyph.unpositioned().h_metrics();
    let advance_width = glyph_metrics.advance_width.round() as i32;
    let left_side_bearing = glyph_metrics.left_side_bearing.round() as i32;

    // Calculate inner rect
    // TODO(JaSc): Explain what inner/outer rects are
    let inner_rect = if let Some(bounding_box) = glyph.pixel_bounding_box() {
        Rect {
            x: bounding_box.min.x + left_side_bearing + padding,
            y: bounding_box.min.y + descent + padding,
            width: bounding_box.width() + padding,
            height: bounding_box.height() + padding,
        }
    } else {
        Rect {
            x: 0 + padding,
            y: 0 + padding,
            width: advance_width + padding,
            height: text_height + padding,
        }
    };
    assert!(
        inner_rect.x >= 0,
        // NOTE: If we ever reach here, it means we need to overhaul our font rendering to
        //       incorporate negative horizontal offsets. It would mean that the left-most
        //       pixel of the glyph is outside of the left outer_rect boundary
        "The x offset of code-point {} was less than zero ({})",
        code_point,
        inner_rect.x
    );

    // Calculate the outer rect / packed rect
    let pack_width = advance_width + 2 * padding;
    let pack_height = text_height + 2 * padding;
    let outer_rect = packer
        .pack(pack_width, pack_height, false)
        .unwrap_or_else(|| {
            panic!(
                "Not enough space to pack glyph for code_point: {}",
                code_point
            )
        });

    (code_point, glyph, inner_rect, outer_rect)
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

fn clear_image(
    image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
    fill_color: [u8; 4],
) {
    let rect = Rect::new(0, 0, image.width() as i32, image.height() as i32);
    fill_rect(image, rect, fill_color);
}

fn fill_rect(
    image: &mut image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>,
    rect: Rect,
    fill_color: [u8; 4],
) {
    for y in rect.y..(rect.y + rect.height) {
        for x in rect.x..(rect.x + rect.width) {
            image.put_pixel(x as u32, y as u32, Rgba { data: fill_color })
        }
    }
}
