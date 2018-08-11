use common::*;
use game_lib::{Bounds, FontHeader, Sprite};

use std;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use bincode::serialize;
use failure;
use failure::{Error, ResultExt};
use image::{DynamicImage, Rgba};
use rand::prelude::*;
use rect_packer::{DensePacker, Rect};
use rusttype::{point, Font, PositionedGlyph, Scale};

const COLOR_GLYPH: [u8; 4] = [255, 255, 255, 255];
const COLOR_BORDER: [u8; 4] = [0, 0, 0, 255];

struct GlyphData<'a> {
    code_point: char,
    glyph: PositionedGlyph<'a>,
    inner_rect: Rect,
    outer_rect: Rect,
}

pub fn pack_font(
    font_filepath: &PathBuf,
    font_height: f32,
    do_draw_borders: bool,
    show_debug_colors: bool,
) -> Result<(), Error> {
    let font_filename = filepath_to_filename_string(font_filepath)?;
    debug!("Packing font: {}", &font_filename);

    // Define input and output file-paths
    let output_dir = Path::new(DATA_DIR).join(FONTS_DIR);
    let texture_filepath = output_dir
        .clone()
        .join(&font_filename)
        .with_extension("png");
    let meta_filepath = output_dir
        .clone()
        .join(&font_filename)
        .with_extension("fnt");
    std::fs::create_dir_all(output_dir.clone())
        .context(format!("Could not create dir '{}'", output_dir.display()))?;

    // Configuration
    let padding = if do_draw_borders { 1 } else { 0 };
    let first_code_point: u8 = 32;
    let last_code_point: u8 = 126;

    // Create font from binary data
    let font_data = {
        let mut font_data = Vec::new();
        File::open(font_filepath)
            .context(format!(
                "Could not open font file '{}'",
                font_filepath.display()
            ))?
            .read_to_end(&mut font_data)
            .context(format!(
                "Could not read font file '{}'",
                font_filepath.display()
            ))?;
        font_data
    };
    let font = Font::from_bytes(&font_data).context("Could not construct front from bytes")?;

    // Rectangle pack glyphs
    let code_points: Vec<char> = (first_code_point..=last_code_point)
        .map(|byte| byte as char)
        .collect();
    let mut packer = DensePacker::new(128, 128);
    let glyph_data = code_points
        .iter()
        .map(|&code_point| pack_glyph(&font, &mut packer, code_point, font_height, padding))
        .collect::<Result<Vec<_>, _>>()
        .context("Could not pack some glyphs")?;
    let (image_width, image_height) = packer.size();

    // Write font and glyph metadata
    write_metadata(
        &meta_filepath,
        &glyph_data,
        image_width as f32,
        image_height as f32,
        first_code_point,
    ).context(format!(
        "Could not write metadata '{}'",
        meta_filepath.display()
    ))?;

    // Create and write out image
    let image = create_font_atlas_texture(
        image_width as u32,
        image_height as u32,
        &glyph_data,
        do_draw_borders,
        padding,
        show_debug_colors,
        &font_filename,
    );
    image.save(texture_filepath.clone()).context(format!(
        "Could not save font texture to '{}'",
        texture_filepath.display()
    ))?;

    info!("Succesfully packed font: '{}'", font_filename);
    Ok(())
}

fn pack_glyph<'a>(
    font: &'a Font,
    packer: &mut DensePacker,
    code_point: char,
    font_height: f32,
    padding: i32,
) -> Result<GlyphData<'a>, Error> {
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
            x: padding,
            y: padding,
            width: advance_width + padding,
            height: text_height + padding,
        }
    };
    assert!(
        inner_rect.x >= 0,
        // NOTE: If we ever reach here, it means we need to overhaul our font rendering to
        //       incorporate negative horizontal offsets. It would mean that the left-most
        //       pixel of the glyph is outside of the left outer_rect boundary
        "The x offset of code-point '{}' was less than zero ({})",
        code_point,
        inner_rect.x
    );

    // Calculate the outer rect / packed rect
    let pack_width = advance_width + 2 * padding;
    let pack_height = text_height + 2 * padding;
    let outer_rect = packer
        .pack(pack_width, pack_height, false)
        .ok_or(failure::err_msg(format!(
            "Not enough space to pack glyph for code_point: '{}'",
            code_point
        )))?;

    Ok(GlyphData {
        code_point,
        glyph,
        inner_rect,
        outer_rect,
    })
}

fn write_metadata(
    meta_filepath: &Path,
    glyph_data: &[GlyphData],
    image_width: f32,
    image_height: f32,
    first_code_point: u8,
) -> Result<(), Error> {
    let mut meta_file = File::create(meta_filepath).context(format!(
        "Could not create file '{}'",
        meta_filepath.display()
    ))?;

    let font_header = FontHeader {
        num_glyphs: glyph_data.len(),
        first_code_point,
    };
    let sprites: Vec<_> = glyph_data
        .iter()
        .map(|data| {
            let rect = data.outer_rect;
            let vertex_bounds = Bounds {
                left: 0.0,
                right: rect.width as f32,
                bottom: 0.0,
                top: rect.height as f32,
            };
            let uv_bounds = Bounds {
                left: rect.x as f32 / image_width,
                right: (rect.x + rect.width) as f32 / image_width,
                bottom: rect.y as f32 / image_height,
                top: (rect.y + rect.height) as f32 / image_height,
            };
            Sprite {
                vertex_bounds,
                uv_bounds,
            }
        })
        .collect();

    let encoded_header = serialize(&font_header).context("Could not encode font metadata header")?;
    meta_file
        .write_all(&encoded_header)
        .context("Could not write font metadata header")?;

    let encoded_sprites = serialize(&sprites).context("Could not encode glyph metadata")?;
    meta_file
        .write_all(&encoded_sprites)
        .context("Could not write glyph metadata")?;

    Ok(())
}

fn create_font_atlas_texture(
    image_width: u32,
    image_height: u32,
    glyph_data: &[GlyphData],
    do_draw_borders: bool,
    padding: i32,
    show_debug_colors: bool,
    font_filename: &str,
) -> Image {
    let mut image = DynamicImage::new_rgba8(image_width, image_height).to_rgba();
    if show_debug_colors {
        image.clear([123, 200, 250, 100]);
    }

    // Draw glyphs
    for data in glyph_data {
        let code_point = data.code_point;
        let glyph = &data.glyph;
        let inner_rect = data.inner_rect;
        let outer_rect = data.outer_rect;

        if show_debug_colors {
            // Visualize outer rect
            let rand_color = [random::<u8>(), random::<u8>(), random::<u8>(), 125];
            image.fill_rect(outer_rect, rand_color);

            // Visualize inner rect
            let rand_color = [random::<u8>(), random::<u8>(), random::<u8>(), 255];
            let inner_rect_with_padding = Rect {
                x: inner_rect.x + outer_rect.x - padding,
                y: inner_rect.y + outer_rect.y - padding,
                width: inner_rect.width + padding,
                height: inner_rect.height + padding,
            };
            image.fill_rect(inner_rect_with_padding, rand_color);
        }

        // Draw actual glyphs
        let glyph_origin_x = (outer_rect.x + inner_rect.x) as u32;
        let glyph_origin_y = (outer_rect.y + inner_rect.y) as u32;
        trace!(
            concat!(
                "\nDrawing glyph '{}' for {} at\n",
                "pos: {} x {}, dim: {} x {}"
            ),
            code_point,
            font_filename,
            glyph_origin_x,
            glyph_origin_y,
            outer_rect.width,
            outer_rect.height
        );
        glyph.draw(|x, y, v| {
            if v > 0.5 {
                image.put_pixel(
                    x + glyph_origin_x,
                    y + glyph_origin_y,
                    Rgba { data: COLOR_GLYPH },
                )
            }
        });
    }
    if do_draw_borders {
        draw_glyph_borders(&mut image, COLOR_GLYPH, COLOR_BORDER);
    }
    image
}

fn draw_glyph_borders(image: &mut Image, color_glyph: [u8; 4], color_border: [u8; 4]) {
    // Create a border around every glyph in the image
    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel_val = *image.get_pixel(x, y);
            if pixel_val.data == color_glyph {
                // We landed on a glyph's pixel. We need to paint a border in our neighbouring
                // pixels that are not themselves part of a glyph
                let pairs = vec![(-1, 0), (1, 0), (0, -1), (0, 1), (1, 1)];
                for pair in pairs {
                    let neighbour_x = (x as i32 + pair.0) as u32;
                    let neighbour_y = (y as i32 + pair.1) as u32;
                    let neighbour_pixel_val = *image.get_pixel(neighbour_x, neighbour_y);

                    if neighbour_pixel_val.data != color_glyph {
                        image.put_pixel(neighbour_x, neighbour_y, Rgba { data: color_border })
                    }
                }
            }
        }
    }
}
