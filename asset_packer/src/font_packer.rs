use common::*;
use common::{AtlasPacker, AtlasRegion};
use game_lib;
use game_lib::{Font, Glyph, ResourcePath, Sprite, Vec2};
use image_packer;

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
use ron;
use rusttype;
use rusttype::{point, PositionedGlyph, Scale};
use std::collections::HashMap;

const COLOR_GLYPH: [u8; 4] = [0, 255, 255, 255];
const COLOR_BORDER: [u8; 4] = [255, 0, 0, 255];
const FIRST_VISIBLE_ASCII_CODE_POINT: u8 = 32;
const LAST_ASCII_CODE_POINT: u8 = 126;
const DEBUG_DRAW_COLORED_GLYPHS: bool = false;

const FONTS_INFO_FILE_PATH: &str = "assets/fonts/fonts.ron";
type FontInfoMap = HashMap<ResourcePath, FontInfo>;

#[derive(Debug, Serialize, Deserialize)]
struct FontInfo {
    filename: String,
    pixel_height: i32,
    border_thickness: u8,
    atlas_padding: u8,
}

#[derive(Debug)]
pub struct RawFont {
    font_height: i32,
    vertical_advance: i32,
    glyphs: Vec<RawGlyph>,
}

#[derive(Debug)]
pub struct RawGlyph {
    code_point: char,
    horizontal_advance: i32,
    offset: (i32, i32),
    image: Option<Image>,
}

pub fn pack_fonts(packer: &mut AtlasPacker) -> Result<HashMap<ResourcePath, Font>, Error> {
    debug!("Reading font-info map file");
    let font_info_map = load_font_info_map_from_file(FONTS_INFO_FILE_PATH)?;
    trace!("FontInfo map: {:?}", font_info_map);

    let mut font_map = HashMap::new();
    for (font_name, font_info) in font_info_map {
        let font = create_font(
            &font_info.filename,
            font_info.pixel_height,
            font_info.border_thickness,
            font_info.atlas_padding,
        ).context(format!(
            "Could not create font '{}' with file '{}'",
            font_name, &font_info.filename
        ))?;

        let packed_font = pack_font(packer, font);
        font_map.insert(font_name, packed_font);
    }
    Ok(font_map)
}

pub fn pack_font(packer: &mut AtlasPacker, raw_font: RawFont) -> Font {
    let packed_glyphs = raw_font
        .glyphs
        .into_iter()
        .map(|raw_glyph| pack_glyph(packer, raw_glyph))
        .collect();

    Font {
        font_height: raw_font.font_height as f32,
        vertical_advance: raw_font.vertical_advance as f32,
        glyphs: packed_glyphs,
    }
}

fn pack_glyph(packer: &mut AtlasPacker, raw_glyph: RawGlyph) -> Glyph {
    let offset = Vec2::new(raw_glyph.offset.0 as f32, raw_glyph.offset.1 as f32);
    let region = if let Some(image) = raw_glyph.image {
        packer.pack_image(image)
    } else {
        packer.default_empty_region()
    };

    let sprite = region.to_sprite(packer.atlas_size as f32, offset);
    let horizontal_advance = raw_glyph.horizontal_advance as f32;

    Glyph {
        sprite,
        horizontal_advance,
    }
}

pub fn create_font(
    font_filename: &str,
    font_height: i32,
    border_thickness: u8,
    atlas_padding: u8,
) -> Result<RawFont, Error> {
    let font_filepath = Path::new(ASSETS_DIR).join(FONTS_DIR).join(font_filename);
    let font = load_font_from_file(font_filepath.to_str().unwrap())?;

    // Font metrics
    let metrics = font.v_metrics(Scale::uniform(font_height as f32));
    let vertical_advance = (metrics.ascent - metrics.descent + metrics.line_gap).ceil() as i32;
    let descent = metrics.descent.ceil() as i32;

    // NOTE: We want to turn ASCII characters 0..127 into glyphs but want to treat the
    //       non-displayable characters 0..31 as just whitespace. So we repeat the whitespace
    //       character 32 times and chain it to the remaining ASCII characters.
    //       The reason we want to treat the non-displayable characters as whitespace is that
    //       if we just use their corresponding codepoints, the glyph will draw unwanted
    //       '▯' symbols instead.
    let code_points: Vec<char> = std::iter::repeat(' ')
        .take(FIRST_VISIBLE_ASCII_CODE_POINT as usize)
        .chain((FIRST_VISIBLE_ASCII_CODE_POINT..=LAST_ASCII_CODE_POINT).map(|byte| byte as char))
        .collect();

    // Create glyphs
    let glyphs: Vec<RawGlyph> = code_points
        .iter()
        .map(|&code_point| {
            create_glyph(
                &font,
                code_point,
                font_height,
                descent,
                border_thickness,
                atlas_padding,
            )
        })
        .collect();

    Ok(RawFont {
        font_height: font_height + 2 * border_thickness as i32,
        vertical_advance: vertical_advance + 2 * border_thickness as i32,
        glyphs,
    })
}

fn create_glyph(
    font: &rusttype::Font,
    code_point: char,
    font_height: i32,
    descent: i32,
    border_thickness: u8,
    atlas_padding: u8,
) -> RawGlyph {
    let glyph = font
            .glyph(code_point)
            .standalone()
            .scaled(Scale::uniform(font_height as f32))
            // NOTE: We pre-position the glyph such that it vertically fits into the 
            //       interval [0, pixel_text_height - 1], where 0 is a glyphs highest possible
            //       point, (pixel_text_height - 1) is a glyphs lowest possible point and
            //       (pixel_text_height - 1 + pixel_descent) represents the fonts baseline.
            .positioned(point(0.0, (descent + font_height) as f32));

    // Glyph metrics
    let metrics = glyph.unpositioned().h_metrics();
    let horizontal_advance = metrics.advance_width.round() as i32 + 2 * border_thickness as i32;
    // NOTE: The offset determines how many pixels the glyph-sprite needs to be offset
    //       from its origin (top-left corner) when drawn to the screen
    let mut offset_x = metrics.left_side_bearing.round() as i32 - (atlas_padding as i32);
    let mut offset_y = -(atlas_padding as i32);

    let maybe_image = create_glyph_image(&glyph, border_thickness, atlas_padding);
    if maybe_image.is_some() {
        // NOTE: We can unwrap here because otherwise maybe_image would be `None` anyway
        let bounding_box = glyph.pixel_bounding_box().unwrap();
        offset_x += bounding_box.min.x;
        offset_y += bounding_box.min.y;
    }

    RawGlyph {
        code_point,
        horizontal_advance,
        offset: (offset_x, offset_y),
        image: maybe_image,
    }
}

fn load_font_info_map_from_file(font_info_filepath: &str) -> Result<FontInfoMap, Error> {
    let file = File::open(font_info_filepath).context("Could not open font-info file")?;
    let map: FontInfoMap = ron::de::from_reader(file).context("Could not read font-info map")?;
    Ok(map)
}

fn load_font_from_file(font_filepath: &str) -> Result<rusttype::Font, Error> {
    debug!("Loading font from file: {}", &font_filepath);
    let font_bytes = std::fs::read(&font_filepath).context("Could not read bytes of font file")?;
    Ok(rusttype::Font::from_bytes(font_bytes).context("Could not construct font from bytes")?)
}

fn create_glyph_image(
    glyph: &PositionedGlyph,
    border_thickness: u8,
    atlas_padding: u8,
) -> Option<Image> {
    glyph.pixel_bounding_box().map(|bounding_box| {
        let mut image = DynamicImage::new_rgba8(
            bounding_box.width() as u32 + 2 * (atlas_padding + border_thickness) as u32,
            bounding_box.height() as u32 + 2 * (atlas_padding + border_thickness) as u32,
        ).to_rgba();

        glyph.draw(|x, y, v| {
            // NOTE: We only use the values that are above 50% opacity and draw them with full
            //       intensity. This way we get nice and crisp edges and a uniform color.
            // WARNING: This only works for pixel-fonts. Regular fonts are not supported
            if v > 0.5 {
                image.put_pixel(
                    x + (atlas_padding + border_thickness) as u32,
                    y + (atlas_padding + border_thickness) as u32,
                    Rgba { data: COLOR_GLYPH },
                )
            }
        });

        if border_thickness != 0 {
            if border_thickness == 1 {
                draw_glyph_border(&mut image, COLOR_GLYPH, COLOR_BORDER);
            } else {
                unimplemented!()
            }
        }

        image
    })
}

fn draw_glyph_border(image: &mut Image, color_glyph: [u8; 4], color_border: [u8; 4]) {
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

/*
////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////
fn pack_font(font_filepath: &str, font_height: f32, do_draw_borders: bool) -> Result<(), Error> {
    let font_filepath = PathBuf::new().join(font_filepath);
    let font_filename = filepath_to_filename_string(&font_filepath)?;
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
    let last_code_point: u8 = LAST_ASCII_CODE_POINT;

    // Create font from binary data
    let font_data = {
        let mut font_data = Vec::new();
        File::open(&font_filepath)
            .context(format!(
                "Could not open font file '{}'",
                &font_filepath.display()
            ))?
            .read_to_end(&mut font_data)
            .context(format!(
                "Could not read font file '{}'",
                &font_filepath.display()
            ))?;
        font_data
    };
    let font =
        rusttype::Font::from_bytes(&font_data).context("Could not construct front from bytes")?;

    // Rectangle pack glyphs
    let code_points: Vec<char> = (FIRST_VISIBLE_ASCII_CODE_POINT..=last_code_point)
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
        DEBUG_DRAW_COLORED_GLYPHS,
        &font_filename,
    );
    image.save(texture_filepath.clone()).context(format!(
        "Could not save font texture to '{}'",
        texture_filepath.display()
    ))?;

    info!("Succesfully packed font: '{}'", font_filename);
    Ok(())
}

struct GlyphData<'a> {
    code_point: char,
    glyph: PositionedGlyph<'a>,
    inner_rect: Rect,
    outer_rect: Rect,
}

fn pack_glyph<'a>(
    font: &'a rusttype::Font,
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
    let outer_rect = packer.pack(pack_width, pack_height, false).ok_or_else(|| {
        failure::err_msg(format!(
            "Not enough space to pack glyph for code_point: '{}'",
            code_point
        ))
    })?;

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
) -> Result<(), Error> {
    let mut meta_file = File::create(meta_filepath).context(format!(
        "Could not create file '{}'",
        meta_filepath.display()
    ))?;

    let mut sprites: Vec<_> = glyph_data
        .iter()
        .map(|data| {
            let rect = data.outer_rect;
            let vertex_bounds = game_lib::Rect {
                left: 0.0,
                right: rect.width as f32,
                bottom: 0.0,
                top: rect.height as f32,
            };
            let uv_bounds = game_lib::Rect {
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

    let sprites = {
        assert_eq!(FIRST_VISIBLE_ASCII_CODE_POINT, 32);
        // We prepend `FIRST_VISIBLE_ASCII_CODE_POINT = 32` number of empty sprites into sprite
        // vector such that the vector index of the code points correspond to their respective
        // ASCII index.
        //
        // Example:
        // Before prepending we have i.e. 'A' (which is 65 in ASCII) at vector index 33.
        // After prepending 32 empty codepoints into the vector we have 'A' at vector index 65,
        // which is now equal to its ASCII-index.
        let num_code_points_before_first_code_point = (FIRST_VISIBLE_ASCII_CODE_POINT as usize) - 1;
        let index_for_space_code_point = (b' ' - FIRST_VISIBLE_ASCII_CODE_POINT) as usize;
        let space_code_point_sprite = sprites[index_for_space_code_point];
        let mut sprites_till_first_codepoint: Vec<_> = std::iter::repeat(space_code_point_sprite)
            .take(num_code_points_before_first_code_point)
            .collect();
        sprites_till_first_codepoint.append(&mut sprites);
        sprites_till_first_codepoint
    };

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
        draw_glyph_border(&mut image, COLOR_GLYPH, COLOR_BORDER);
    }
    image
}
*/
