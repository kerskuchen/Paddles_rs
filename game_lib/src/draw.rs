use math::{Bounds, Color, Line, Mat4, Point, Rect, Vec2, WorldPoint};

use bincode;
use lodepng;
use rgb;
pub use rgb::ComponentBytes;

use std;
use std::collections::HashMap;
use std::fs::File;

pub type Pixel = rgb::RGBA8;
pub type VertexIndex = u16;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

//==================================================================================================
// DrawContext
//==================================================================================================
//

// TODO(JaSc): Change screen color based on debug/release to better see
//             letterboxing in windowed mode
const CLEAR_COLOR_SCREEN: [f32; 4] = [0.2, 0.9, 0.4, 1.0];
const CLEAR_COLOR_CANVAS: [f32; 4] = [1.0, 0.4, 0.7, 1.0];

#[derive(Default)]
pub struct DrawContext {
    texture_atlas: Option<TextureInfo>,
    texture_font: Option<TextureInfo>,
    sprite_map: HashMap<String, Sprite>,
    glyph_sprites: Vec<Sprite>,

    canvas_framebuffer: Option<FramebufferInfo>,

    line_batch: LineBatch,
    fill_batch: QuadBatch,

    draw_commands: Vec<DrawCommand>,
}

impl DrawContext {
    pub fn new() -> DrawContext {
        DrawContext {
            texture_atlas: None,
            texture_font: None,
            sprite_map: HashMap::new(),
            glyph_sprites: Vec::new(),

            canvas_framebuffer: None,

            line_batch: LineBatch::new(),
            fill_batch: QuadBatch::new(),

            draw_commands: Vec::new(),
        }
    }

    pub fn draw_line(&mut self, line: Line, depth: f32, color: Color) {
        // TODO(JaSc): Cache the plain texture uv for reuse
        let plain_uv = self.sprite_map["images/plain"].uv_bounds;
        let line_uv = sprite_uv_to_line_uv(plain_uv);
        self.line_batch.push_line(line, line_uv, depth, color);
    }

    pub fn draw_rect_filled(&mut self, bounds: Bounds, depth: f32, color: Color) {
        // TODO(JaSc): Cache the plain texture uv for reuse
        let plain_uv = self.sprite_map["images/plain"].uv_bounds;
        self.fill_batch.push_quad(bounds, plain_uv, depth, color);
    }

    pub fn start_drawing(&mut self) {
        self.fill_batch.clear();
        self.line_batch.clear();
    }

    // TODO(JaSc): Get rid of screen_rect/canvas_rect here
    pub fn finish_drawing(
        &mut self,
        transform: Mat4,
        canvas_rect: Rect,
        canvas_blit_rect: Rect,
    ) -> Vec<DrawCommand> {
        let canvas_framebuffer = self
            .canvas_framebuffer
            .clone()
            .expect("Canvas framebuffer does not exist");
        let texture_atlas = self
            .texture_atlas
            .clone()
            .expect("Texture atlas does not exist");

        // Clear screen and canvas
        self.draw_commands.push(DrawCommand::Clear {
            framebuffer: FramebufferTarget::Screen,
            color: Color::from(CLEAR_COLOR_SCREEN),
        });
        self.draw_commands.push(DrawCommand::Clear {
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            color: Color::from(CLEAR_COLOR_CANVAS),
        });

        // Draw batches
        let (vertices, indices) = self.fill_batch.extract_vertices_indices();
        self.draw_commands.push(DrawCommand::Draw {
            transform,
            texture_info: texture_atlas.clone(),
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            vertices,
            indices,
            draw_mode: DrawMode::Fill,
        });
        let (vertices, indices) = self.line_batch.extract_vertices_indices();
        self.draw_commands.push(DrawCommand::Draw {
            transform,
            texture_info: texture_atlas.clone(),
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            vertices,
            indices,
            draw_mode: DrawMode::Lines,
        });

        // Blit canvas to screen
        self.draw_commands.push(DrawCommand::BlitFramebuffer {
            source_framebuffer: canvas_framebuffer.clone(),
            target_framebuffer: FramebufferTarget::Screen,
            source_rect: canvas_rect,
            target_rect: canvas_blit_rect,
        });

        std::mem::replace(&mut self.draw_commands, Vec::new())
    }

    pub fn reinitialize(&mut self, canvas_width: u16, canvas_height: u16) {
        // -----------------------------------------------------------------------------------------
        // Sprite creation
        //

        // Create atlas sprites from metafile
        let mut atlas_metafile =
            File::open("data/images/atlas.tex").expect("Could not load atlas metafile");
        self.sprite_map = bincode::deserialize_from(&mut atlas_metafile)
            .expect("Could not deserialize sprite map");

        // Delete old texture if it exists
        if let Some(old_atlas_texture_info) = self.texture_atlas.take() {
            self.draw_commands.push(DrawCommand::DeleteTexture {
                texture_info: old_atlas_texture_info,
            });
        }
        // Create atlas texture
        let (texture_info, pixels) = load_texture(0, "data/images/atlas.png");
        self.texture_atlas = Some(texture_info.clone());
        self.draw_commands.push(DrawCommand::CreateTexture {
            texture_info,
            pixels,
        });

        // -----------------------------------------------------------------------------------------
        // Font creation
        //

        // Create font-glyph sprites from metafile
        let mut font_metafile =
            File::open("data/fonts/04B_03__.fnt").expect("Could not load font metafile");
        self.glyph_sprites = bincode::deserialize_from(&mut font_metafile)
            .expect("Could not deserialize font glyphs");

        // Delete old texture if it exists
        if let Some(old_font_texture_info) = self.texture_font.take() {
            self.draw_commands.push(DrawCommand::DeleteTexture {
                texture_info: old_font_texture_info,
            });
        }
        // Create font texture
        let (texture_info, pixels) = load_texture(1, "data/fonts/04B_03__.png");
        self.texture_font = Some(texture_info.clone());
        self.draw_commands.push(DrawCommand::CreateTexture {
            texture_info,
            pixels,
        });

        // -----------------------------------------------------------------------------------------
        // Framebuffer creation
        //

        // Delete old framebuffer if it exists
        if let Some(old_canvas_framebuffer_info) = self.canvas_framebuffer.take() {
            self.draw_commands.push(DrawCommand::DeleteFramebuffer {
                framebuffer_info: old_canvas_framebuffer_info,
            });
        }

        // Create new canvas framebuffer
        let framebuffer_info = FramebufferInfo {
            id: 0,
            width: canvas_width,
            height: canvas_height,
            name: String::from("Canvas"),
        };
        self.canvas_framebuffer = Some(framebuffer_info.clone());
        self.draw_commands
            .push(DrawCommand::CreateFramebuffer { framebuffer_info });
    }
}

fn load_texture(id: u32, file_name: &str) -> (TextureInfo, Vec<rgb::RGBA8>) {
    let image = lodepng::decode32_file(file_name).unwrap();
    let texture_info = TextureInfo {
        id,
        width: image.width as u16,
        height: image.height as u16,
        name: String::from(file_name),
    };
    (texture_info, image.buffer)
}

//==================================================================================================
// DrawCommand
//==================================================================================================
//
pub enum DrawCommand {
    Draw {
        transform: Mat4,
        vertices: Vec<Vertex>,
        indices: Vec<VertexIndex>,
        texture_info: TextureInfo,
        framebuffer: FramebufferTarget,
        draw_mode: DrawMode,
    },
    Clear {
        framebuffer: FramebufferTarget,
        color: Color,
    },
    BlitFramebuffer {
        source_framebuffer: FramebufferInfo,
        target_framebuffer: FramebufferTarget,
        source_rect: Rect,
        target_rect: Rect,
    },
    CreateFramebuffer {
        framebuffer_info: FramebufferInfo,
    },
    DeleteFramebuffer {
        framebuffer_info: FramebufferInfo,
    },
    CreateTexture {
        texture_info: TextureInfo,
        pixels: Vec<Pixel>,
    },
    DeleteTexture {
        texture_info: TextureInfo,
    },
}

impl std::fmt::Debug for DrawCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DrawCommand::Draw {
                transform,
                vertices,
                texture_info,
                framebuffer,
                draw_mode,
                ..
            } => write!(
                f,
                "\n  Draw:\n  {:?}\n  {:?}\n  num_verts: {:?}\n  {:?}\n  {:?}",
                draw_mode,
                transform,
                vertices.len(),
                texture_info,
                framebuffer
            ),
            DrawCommand::CreateTexture {
                texture_info,
                pixels,
            } => write!(
                f,
                "\n  CreateTexture:\n  {:?}\n  num_pixels: {:?}",
                texture_info,
                pixels.len()
            ),
            DrawCommand::Clear { framebuffer, color } => {
                write!(f, "\n  Clear: color: {:?}, {:?}", color, framebuffer)
            }
            DrawCommand::BlitFramebuffer {
                source_framebuffer,
                target_framebuffer,
                source_rect,
                target_rect,
            } => write!(
                f,
                concat!(
                    "\n  BlitFramebuffer:\n  source: {:?}",
                    "\n  target: {:?}\n  source: {:?}\n  target: {:?}"
                ),
                source_framebuffer,
                target_framebuffer,
                source_rect,
                target_rect,
            ),
            DrawCommand::CreateFramebuffer { framebuffer_info } => {
                write!(f, "\n  CreateFramebuffer: {:?}", framebuffer_info,)
            }
            DrawCommand::DeleteFramebuffer { framebuffer_info } => {
                write!(f, "\n  DeleteFramebuffer: {:?}", framebuffer_info,)
            }
            DrawCommand::DeleteTexture { texture_info } => {
                write!(f, "\n  DeleteTexture: {:?}", texture_info,)
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DrawMode {
    Lines,
    Fill,
}

#[derive(Debug, Clone)]
pub enum FramebufferTarget {
    Screen,
    Offscreen(FramebufferInfo),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureInfo {
    pub id: u32,
    pub width: u16,
    pub height: u16,
    pub name: String,
}

impl TextureInfo {
    pub fn empty() -> TextureInfo {
        TextureInfo {
            id: 0,
            width: 0,
            height: 0,
            name: String::from(""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FramebufferInfo {
    pub id: u32,
    pub width: u16,
    pub height: u16,
    pub name: String,
}

impl FramebufferInfo {
    pub fn empty() -> FramebufferInfo {
        FramebufferInfo {
            id: 0,
            width: 0,
            height: 0,
            name: String::from(""),
        }
    }
}

//==================================================================================================
// Batch drawing
//==================================================================================================
//

// -------------------------------------------------------------------------------------------------
// Quads
//
#[derive(Default)]
pub struct QuadBatch {
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
}

impl QuadBatch {
    const VERTICES_PER_QUAD: usize = 4;
    const INDICES_PER_QUAD: usize = 6;

    pub fn new() -> QuadBatch {
        QuadBatch {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        // NOTE: We don't clear the indices as we want to reuse them. This is possible because we
        //       know that we always will only store quads in this batch.
        self.vertices.clear();
    }

    pub fn push_quad(&mut self, bounds: Bounds, bounds_uv: Bounds, depth: f32, color: Color) {
        let color = color.into();

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let quad_vertices = [
            Vertex {
                pos: [bounds.left, bounds.bottom, depth, 1.0],
                uv: [bounds_uv.left, bounds_uv.top],
                color,
            },
            Vertex {
                pos: [bounds.right, bounds.bottom, depth, 1.0],
                uv: [bounds_uv.right, bounds_uv.top],
                color,
            },
            Vertex {
                pos: [bounds.right, bounds.top, depth, 1.0],
                uv: [bounds_uv.right, bounds_uv.bottom],
                color,
            },
            Vertex {
                pos: [bounds.left, bounds.top, depth, 1.0],
                uv: [bounds_uv.left, bounds_uv.bottom],
                color,
            },
        ];
        self.vertices.extend_from_slice(&quad_vertices);
    }

    pub fn extract_vertices_indices(&mut self) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let num_quads = self.vertices.len() / QuadBatch::VERTICES_PER_QUAD;
        let num_indices_to_fill = (num_quads * QuadBatch::INDICES_PER_QUAD) as VertexIndex;
        let num_indices_already_filled = self.indices.len() as VertexIndex;

        // Fill our indices vector if needed
        if num_indices_already_filled < num_indices_to_fill {
            for quad_index in num_indices_already_filled..num_indices_to_fill {
                let quad_indices = [
                    4 * quad_index,
                    4 * quad_index + 1,
                    4 * quad_index + 2,
                    4 * quad_index + 2,
                    4 * quad_index + 3,
                    4 * quad_index,
                ];
                self.indices.extend(&quad_indices);
            }
        }

        // TODO(JaSc): Return references
        (
            self.vertices.clone(),
            self.indices
                .clone()
                .into_iter()
                .take(num_indices_to_fill as usize)
                .collect(),
        )
    }
}

// -------------------------------------------------------------------------------------------------
// Lines
//
#[derive(Default)]
pub struct LineBatch {
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
}

impl LineBatch {
    const VERTICES_PER_LINE: usize = 2;
    const INDICES_PER_LINE: usize = 2;

    pub fn new() -> LineBatch {
        LineBatch {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
    pub fn clear(&mut self) {
        // NOTE: We don't clear the indices as we want to reuse them. This is possible because we
        //       know that we always will store only line in this batch.
        self.vertices.clear();
    }

    pub fn push_line(&mut self, line: Line, line_uv: Line, depth: f32, color: Color) {
        let color = color.into();
        let line_vertices = [
            Vertex {
                pos: [line.start.x, line.start.y, depth, 1.0],
                uv: [line_uv.start.x, line_uv.start.y],
                color,
            },
            Vertex {
                pos: [line.end.x, line.end.y, depth, 1.0],
                uv: [line_uv.end.x, line_uv.end.y],
                color,
            },
        ];
        self.vertices.extend_from_slice(&line_vertices);
    }

    pub fn extract_vertices_indices(&mut self) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let num_lines = self.vertices.len() / LineBatch::VERTICES_PER_LINE;
        let num_indices_to_fill = (num_lines * LineBatch::INDICES_PER_LINE) as VertexIndex;
        let num_indices_already_filled = self.indices.len() as VertexIndex;

        // Fill our indices vector if needed
        if num_indices_already_filled < num_indices_to_fill {
            for line_index in num_indices_already_filled..num_indices_to_fill {
                let line_indices = [2 * line_index, 2 * line_index + 1];
                self.indices.extend(&line_indices);
            }
        }

        // TODO(JaSc): Return references
        (
            self.vertices.clone(),
            self.indices
                .clone()
                .into_iter()
                .take(num_indices_to_fill as usize)
                .collect(),
        )
    }
}

//==================================================================================================
// Sprite
//==================================================================================================
//

// TODO(JaSc): Evaluate if we still need this struct or a better alternative/name
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Sprite {
    pub vertex_bounds: Bounds,
    pub uv_bounds: Bounds,
}

impl Sprite {
    pub fn with_scale(self, scale: Vec2) -> Sprite {
        Sprite {
            vertex_bounds: self.vertex_bounds.scaled_from_origin(scale),
            uv_bounds: self.uv_bounds,
        }
    }

    pub fn into_vertices(self, pos: WorldPoint, depth: f32, color: Color) -> [Vertex; 4] {
        let vertex = self.vertex_bounds;
        let uv = self.uv_bounds;
        let color = color.into();

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        [
            Vertex {
                pos: [pos.x + vertex.left, pos.y + vertex.bottom, depth, 1.0],
                uv: [uv.left, uv.top],
                color,
            },
            Vertex {
                pos: [pos.x + vertex.right, pos.y + vertex.bottom, depth, 1.0],
                uv: [uv.right, uv.top],
                color,
            },
            Vertex {
                pos: [pos.x + vertex.right, pos.y + vertex.top, depth, 1.0],
                uv: [uv.right, uv.bottom],
                color,
            },
            Vertex {
                pos: [pos.x + vertex.left, pos.y + vertex.top, depth, 1.0],
                uv: [uv.left, uv.bottom],
                color,
            },
        ]
    }
}

//==================================================================================================
// Quad
//==================================================================================================
//
// TODO(JaSc): Evaluate if we still need this struct
#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub bounds: Bounds,
    pub depth: f32,
    pub color: Color,
}

impl Quad {
    pub fn from_rect(rect: Rect, depth: f32, color: Color) -> Quad {
        Quad {
            bounds: rect.to_bounds(),
            depth,
            color,
        }
    }

    pub fn from_rect_centered(rect: Rect, depth: f32, color: Color) -> Quad {
        Quad {
            bounds: rect.to_bounds_centered(),
            depth,
            color,
        }
    }

    pub fn from_bounds(bounds: Bounds, depth: f32, color: Color) -> Quad {
        Quad {
            bounds,
            depth,
            color,
        }
    }

    pub fn into_vertices(self) -> [Vertex; 4] {
        let bounds = self.bounds;
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        [
            Vertex {
                pos: [bounds.left, bounds.bottom, depth, 1.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                pos: [bounds.right, bounds.bottom, depth, 1.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                pos: [bounds.right, bounds.top, depth, 1.0],
                uv: [1.0, 0.0],
                color,
            },
            Vertex {
                pos: [bounds.left, bounds.top, depth, 1.0],
                uv: [0.0, 0.0],
                color,
            },
        ]
    }
}

//==================================================================================================
// Helper functions
//==================================================================================================
//
// TODO(JaSc): Find a place for these graphic/geometry helper functions
fn sprite_uv_to_line_uv(sprite_uv: Bounds) -> Line {
    // NOTE: We use only the horizontal axis of a sprite's uv
    Line::new(
        Point::new(sprite_uv.left, sprite_uv.bottom),
        Point::new(sprite_uv.right, sprite_uv.top),
    )
}
