use common::AtlasPacker;
use common::*;
use game_lib::{Font, Glyph, ResourcePath, Vec2};

use std;
use std::fs::File;
use std::path::Path;

use failure::{Error, ResultExt};
use image::{DynamicImage, Rgba};
use ron;
use rusttype;
use rusttype::{point, PositionedGlyph, Scale};
use std::collections::HashMap;

const COLOR_GLYPH: [u8; 4] = [0, 255, 255, 255];
const COLOR_BORDER: [u8; 4] = [255, 0, 0, 255];
const FIRST_VISIBLE_ASCII_CODE_POINT: u8 = 32;
const LAST_ASCII_CODE_POINT: u8 = 126;

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
        font_height: font_height + 2 * i32::from(border_thickness),
        vertical_advance: vertical_advance + 2 * i32::from(border_thickness),
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
    let horizontal_advance = metrics.advance_width.round() as i32 + 2 * i32::from(border_thickness);
    // NOTE: The offset determines how many pixels the glyph-sprite needs to be offset
    //       from its origin (top-left corner) when drawn to the screen
    let mut offset_x = metrics.left_side_bearing.round() as i32 - i32::from(atlas_padding);
    let mut offset_y = -i32::from(atlas_padding);

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
            bounding_box.width() as u32 + 2 * u32::from(atlas_padding + border_thickness),
            bounding_box.height() as u32 + 2 * u32::from(atlas_padding + border_thickness),
        ).to_rgba();

        glyph.draw(|x, y, v| {
            // NOTE: We only use the values that are above 50% opacity and draw them with full
            //       intensity. This way we get nice and crisp edges and a uniform color.
            // WARNING: This only works for pixel-fonts. Regular fonts are not supported
            if v > 0.5 {
                image.put_pixel(
                    x + u32::from(atlas_padding + border_thickness),
                    y + u32::from(atlas_padding + border_thickness),
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
