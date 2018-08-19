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

pub struct DrawContext<'drawcontext> {
    texture_atlas: Option<TextureInfo>,
    texture_font: Option<TextureInfo>,
    sprite_map: HashMap<String, Sprite>,
    glyph_sprites: Vec<Sprite>,

    canvas_framebuffer: Option<FramebufferInfo>,

    mesh_lines: MeshLines,
    mesh_polys: MeshPolys,

    pub draw_commands: Vec<DrawCommand<'drawcontext>>,
}

impl<'drawcontext> DrawContext<'drawcontext> {
    pub fn new() -> DrawContext<'drawcontext> {
        DrawContext {
            texture_atlas: None,
            texture_font: None,
            sprite_map: HashMap::new(),
            glyph_sprites: Vec::new(),

            canvas_framebuffer: None,

            mesh_lines: MeshLines::new(),
            mesh_polys: MeshPolys::new(),

            draw_commands: Vec::new(),
        }
    }

    pub fn draw_line(&mut self, line: Line, depth: f32, color: Color) {
        // TODO(JaSc): Cache the plain texture uv for reuse
        let plain_uv = self.sprite_map["images/plain"].uv_bounds;
        let line_uv = sprite_uv_to_line_uv(plain_uv);
        self.mesh_lines.push_line(line, line_uv, depth, color);
    }

    pub fn draw_rect_filled(&mut self, bounds: Bounds, depth: f32, color: Color) {
        // TODO(JaSc): Cache the plain texture uv for reuse
        let plain_uv = self.sprite_map["images/plain"].uv_bounds;
        self.mesh_polys.push_quad(bounds, plain_uv, depth, color);
    }

    pub fn start_drawing(&mut self) {
        self.mesh_polys.clear();
        self.mesh_lines.clear();
    }

    // TODO(JaSc): Get rid of screen_rect/canvas_rect here
    pub fn finish_drawing(
        &'drawcontext mut self,
        transform: Mat4,
        canvas_rect: Rect,
        canvas_blit_rect: Rect,
    ) {
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
        self.draw_commands.push(DrawCommand::DrawLines {
            transform,
            texture_info: texture_atlas.clone(),
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            mesh_lines: &self.mesh_lines,
        });
        self.draw_commands.push(DrawCommand::DrawPolys {
            transform,
            texture_info: texture_atlas.clone(),
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            mesh_polys: &self.mesh_polys,
        });

        // Blit canvas to screen
        self.draw_commands.push(DrawCommand::BlitFramebuffer {
            source_framebuffer: canvas_framebuffer.clone(),
            target_framebuffer: FramebufferTarget::Screen,
            source_rect: canvas_rect,
            target_rect: canvas_blit_rect,
        });
    }

    pub fn reinitialize(&mut self, canvas_width: u16, canvas_height: u16) {
        self.draw_commands.clear();

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
pub enum DrawCommand<'drawcontext> {
    DrawLines {
        transform: Mat4,
        mesh_lines: &'drawcontext Mesh<GeometryTypeLines>,
        texture_info: TextureInfo,
        framebuffer: FramebufferTarget,
    },
    DrawPolys {
        transform: Mat4,
        mesh_polys: &'drawcontext Mesh<GeometryTypePolys>,
        texture_info: TextureInfo,
        framebuffer: FramebufferTarget,
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

impl<'drawbuffers> std::fmt::Debug for DrawCommand<'drawbuffers> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DrawCommand::DrawLines {
                transform,
                mesh_lines,
                texture_info,
                framebuffer,
                ..
            } => write!(
                f,
                "\n  DrawLines:\n  {:?}\n  num_verts: {:?}\n  {:?}\n  {:?}",
                transform,
                mesh_lines.vertices.len(),
                texture_info,
                framebuffer
            ),
            DrawCommand::DrawPolys {
                transform,
                mesh_polys,
                texture_info,
                framebuffer,
                ..
            } => write!(
                f,
                "\n  DrawPolys:\n  {:?}\n  num_verts: {:?}\n  {:?}\n  {:?}",
                transform,
                mesh_polys.vertices.len(),
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
// Mesh
//==================================================================================================
//

pub type MeshLines = Mesh<GeometryTypeLines>;
pub type MeshPolys = Mesh<GeometryTypePolys>;

pub struct GeometryTypeLines;
pub struct GeometryTypePolys;

#[derive(Clone)]
pub struct Mesh<T> {
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
    geometry_type: std::marker::PhantomData<T>,
}

impl<T> Mesh<T> {
    pub fn new() -> Mesh<T> {
        Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            geometry_type: std::marker::PhantomData,
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn to_vertices_indices(&self) -> (&[Vertex], &[VertexIndex]) {
        (&self.vertices, &self.indices)
    }
}

impl Mesh<GeometryTypeLines> {
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

        let line_vertex_index = self.vertices.len() as VertexIndex;
        let line_indices = [line_vertex_index, line_vertex_index + 1];

        self.vertices.extend_from_slice(&line_vertices);
        self.indices.extend(&line_indices);
    }
}

impl Mesh<GeometryTypePolys> {
    pub fn push_quad(&mut self, bounds: Bounds, bounds_uv: Bounds, depth: f32, color: Color) {
        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let color = color.into();
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

        let quad_vertex_index = self.vertices.len() as VertexIndex;
        let quad_indices = [
            quad_vertex_index,
            quad_vertex_index + 1,
            quad_vertex_index + 2,
            quad_vertex_index + 2,
            quad_vertex_index + 3,
            quad_vertex_index,
        ];

        self.vertices.extend_from_slice(&quad_vertices);
        self.indices.extend(&quad_indices);
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
