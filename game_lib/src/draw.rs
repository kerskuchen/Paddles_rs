use math::{Color, Line, Mat4, Point, Rect, Vec2, WorldPoint};

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
    pub uv: [f32; 3],
    pub color: [f32; 4],
}

pub const COLOR_RED: Color = Color {
    x: 1.0,
    y: 0.0,
    z: 0.0,
    w: 1.0,
};
pub const COLOR_GREEN: Color = Color {
    x: 0.0,
    y: 1.0,
    z: 0.0,
    w: 1.0,
};
pub const COLOR_BLUE: Color = Color {
    x: 0.0,
    y: 0.0,
    z: 1.0,
    w: 1.0,
};
pub const COLOR_CYAN: Color = Color {
    x: 0.0,
    y: 1.0,
    z: 1.0,
    w: 1.0,
};
pub const COLOR_YELLOW: Color = Color {
    x: 1.0,
    y: 1.0,
    z: 0.0,
    w: 1.0,
};
pub const COLOR_MAGENTA: Color = Color {
    x: 1.0,
    y: 0.0,
    z: 1.0,
    w: 1.0,
};
pub const COLOR_BLACK: Color = Color {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 1.0,
};
pub const COLOR_WHITE: Color = Color {
    x: 1.0,
    y: 1.0,
    z: 1.0,
    w: 1.0,
};

//==================================================================================================
// DrawContext
//==================================================================================================
//

// TODO(JaSc): Change screen color based on debug/release to better see
//             letterboxing in windowed mode
const CLEAR_COLOR_SCREEN: [f32; 4] = [0.2, 0.9, 0.4, 1.0];
const CLEAR_COLOR_CANVAS: [f32; 4] = [1.0, 0.4, 0.7, 1.0];

#[derive(Default)]
pub struct DrawContext<'drawcontext> {
    atlas: AtlasMeta,
    atlas_texture_array: Option<TextureArrayInfo>,

    canvas_framebuffer: Option<FramebufferInfo>,

    lines: LineMesh,
    polygons: PolygonMesh,

    pub draw_commands: Vec<DrawCommand<'drawcontext>>,
}

impl<'drawcontext> DrawContext<'drawcontext> {
    pub fn new() -> DrawContext<'drawcontext> {
        Default::default()
    }

    pub fn draw_line(&mut self, line: Line, depth: f32, color: Color) {
        // TODO(JaSc): Cache the plain texture uv for reuse
        let sprite = self.atlas.sprites["images/plain"];
        let line_uv = rect_uv_to_line_uv(sprite.uv_bounds);
        self.lines
            .push_line(line, line_uv, sprite.atlas_index, depth, color);
    }

    pub fn draw_rect_filled(&mut self, rect: Rect, depth: f32, color: Color) {
        // TODO(JaSc): Cache the plain texture uv for reuse
        let sprite = self.atlas.sprites["images/plain"];
        self.polygons
            .push_quad(rect, sprite.uv_bounds, sprite.atlas_index, depth, color);
    }

    pub fn debug_draw_rect_textured(&mut self, rect: Rect, depth: f32, color: Color) {
        let sprite = self.atlas.sprites["images/textured"];
        self.polygons
            .push_quad(rect, sprite.uv_bounds, sprite.atlas_index, depth, color);
    }

    pub fn debug_draw_circle_textured(&mut self, pos: Point, depth: f32, color: Color) {
        let sprite = self.atlas.animations["images/test"].frames[2];
        let vertex_bounds = sprite.vertex_bounds.translated_by(pos);
        self.polygons.push_quad(
            vertex_bounds,
            sprite.uv_bounds,
            sprite.atlas_index,
            depth,
            color,
        );
    }

    pub fn start_drawing(&mut self) {
        self.polygons.clear();
        self.lines.clear();
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
            .atlas_texture_array
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
        self.draw_commands.push(DrawCommand::DrawPolys {
            transform,
            texture_array_info: texture_atlas.clone(),
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            mesh: &self.polygons,
        });
        self.draw_commands.push(DrawCommand::DrawLines {
            transform,
            texture_array_info: texture_atlas.clone(),
            framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
            mesh: &self.lines,
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
        // Create atlas from metafile
        let mut atlas_metafile =
            File::open("data/atlas.tex").expect("Could not load atlas metafile");
        self.atlas = bincode::deserialize_from(&mut atlas_metafile)
            .expect("Could not deserialize sprite map");

        // Delete old atlas textures if they exists
        if let Some(old_atlas_texture_array_info) = self.atlas_texture_array.take() {
            self.draw_commands.push(DrawCommand::DeleteTextureArray {
                texture_array_info: old_atlas_texture_array_info,
            });
        }
        // Create atlas textures
        let (texture_array_info, pixels) =
            load_texture_array(0, "data/atlas", self.atlas.num_atlas_textures);
        self.atlas_texture_array = Some(texture_array_info.clone());

        self.draw_commands.push(DrawCommand::CreateTextureArray {
            texture_array_info,
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

fn load_texture_array(
    id: u32,
    file_name: &str,
    num_textures: usize,
) -> (TextureArrayInfo, Vec<Vec<rgb::RGBA8>>) {
    let mut pixels = Vec::new();
    let mut width = 0;
    let mut height = 0;

    for index in 0..num_textures {
        let file_path = format!("{}_{}.png", file_name, index);
        let image = lodepng::decode32_file(&file_path)
            .unwrap_or_else(|error| panic!("Could not open '{}' : {}", file_path, error));

        // TODO(JaSc): Check that all textures have the same dimensions
        width = image.width;
        height = image.height;
        pixels.push(image.buffer);
    }

    let texture_array_info = TextureArrayInfo {
        id,
        width: width as u16,
        height: height as u16,
        num_textures: num_textures as u16,
        name: String::from(file_name),
    };

    (texture_array_info, pixels)
}

//==================================================================================================
// DrawCommand
//==================================================================================================
//
pub enum DrawCommand<'drawcontext> {
    DrawLines {
        transform: Mat4,
        mesh: &'drawcontext LineMesh,
        texture_array_info: TextureArrayInfo,
        framebuffer: FramebufferTarget,
    },
    DrawPolys {
        transform: Mat4,
        mesh: &'drawcontext PolygonMesh,
        texture_array_info: TextureArrayInfo,
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
    CreateTextureArray {
        texture_array_info: TextureArrayInfo,
        pixels: Vec<Vec<Pixel>>,
    },
    DeleteTextureArray {
        texture_array_info: TextureArrayInfo,
    },
}

impl<'drawbuffers> std::fmt::Debug for DrawCommand<'drawbuffers> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DrawCommand::DrawLines {
                transform,
                mesh,
                texture_array_info,
                framebuffer,
                ..
            } => write!(
                f,
                "\n  DrawLines:\n  {:?}\n  num_verts: {:?}\n  {:?}\n  {:?}",
                transform,
                mesh.vertices.len(),
                texture_array_info,
                framebuffer
            ),
            DrawCommand::DrawPolys {
                transform,
                mesh,
                texture_array_info,
                framebuffer,
                ..
            } => write!(
                f,
                "\n  DrawPolys:\n  {:?}\n  num_verts: {:?}\n  {:?}\n  {:?}",
                transform,
                mesh.vertices.len(),
                texture_array_info,
                framebuffer
            ),
            DrawCommand::CreateTextureArray {
                texture_array_info,
                pixels,
            } => write!(
                f,
                "\n  CreateTexture:\n  {:?}\n  num_pixels: {:?}",
                texture_array_info,
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
            DrawCommand::DeleteTextureArray { texture_array_info } => {
                write!(f, "\n  DeleteTexture: {:?}", texture_array_info,)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum FramebufferTarget {
    Screen,
    Offscreen(FramebufferInfo),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureArrayInfo {
    pub id: u32,
    pub width: u16,
    pub height: u16,
    pub num_textures: u16,
    pub name: String,
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

pub trait Mesh {
    fn new() -> Self;
    fn clear(&mut self);
    fn to_vertices_indices(&self) -> (&[Vertex], &[VertexIndex]);
}

// -------------------------------------------------------------------------------------------------
// LineMesh
//
#[derive(Default)]
pub struct LineMesh {
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
}

impl Mesh for LineMesh {
    fn new() -> Self {
        Default::default()
    }

    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    fn to_vertices_indices(&self) -> (&[Vertex], &[VertexIndex]) {
        (&self.vertices, &self.indices)
    }
}

impl LineMesh {
    pub fn push_line(
        &mut self,
        line: Line,
        line_uv: Line,
        atlas_index: u32,
        depth: f32,
        color: Color,
    ) {
        let color = color.into();
        let atlas_index = atlas_index as f32;
        let line_vertices = [
            Vertex {
                pos: [line.start.x, line.start.y, depth, 1.0],
                uv: [line_uv.start.x, line_uv.start.y, atlas_index],
                color,
            },
            Vertex {
                pos: [line.end.x, line.end.y, depth, 1.0],
                uv: [line_uv.end.x, line_uv.end.y, atlas_index],
                color,
            },
        ];

        let line_vertex_index = self.vertices.len() as VertexIndex;
        let line_indices = [line_vertex_index, line_vertex_index + 1];

        self.vertices.extend_from_slice(&line_vertices);
        self.indices.extend(&line_indices);
    }
}

// -------------------------------------------------------------------------------------------------
// PolygonMesh
//
#[derive(Default)]
pub struct PolygonMesh {
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
}

impl Mesh for PolygonMesh {
    fn new() -> Self {
        Default::default()
    }

    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    fn to_vertices_indices(&self) -> (&[Vertex], &[VertexIndex]) {
        (&self.vertices, &self.indices)
    }
}

impl PolygonMesh {
    pub fn push_quad(
        &mut self,
        rect: Rect,
        rect_uv: Rect,
        atlas_index: u32,
        depth: f32,
        color: Color,
    ) {
        let color = color.into();
        let atlas_index = atlas_index as f32;
        let quad_vertices = [
            Vertex {
                pos: [rect.left, rect.bottom, depth, 1.0],
                uv: [rect_uv.left, rect_uv.bottom, atlas_index],
                color,
            },
            Vertex {
                pos: [rect.right, rect.bottom, depth, 1.0],
                uv: [rect_uv.right, rect_uv.bottom, atlas_index],
                color,
            },
            Vertex {
                pos: [rect.right, rect.top, depth, 1.0],
                uv: [rect_uv.right, rect_uv.top, atlas_index],
                color,
            },
            Vertex {
                pos: [rect.left, rect.top, depth, 1.0],
                uv: [rect_uv.left, rect_uv.top, atlas_index],
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

#[derive(Default, Serialize, Deserialize)]
pub struct AtlasMeta {
    pub num_atlas_textures: usize,
    pub fonts: HashMap<::ResourcePath, Font>,
    pub animations: HashMap<::ResourcePath, Animation>,
    pub sprites: HashMap<::ResourcePath, Sprite>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Animation {
    pub frame_durations: Vec<f32>,
    pub frames: Vec<Sprite>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Font {
    pub font_height: f32,
    pub vertical_advance: f32,
    pub glyphs: Vec<Glyph>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Glyph {
    pub sprite: Sprite,
    pub horizontal_advance: f32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct Sprite {
    pub vertex_bounds: Rect,
    pub uv_bounds: Rect,
    pub atlas_index: u32,
}

impl Sprite {
    pub fn with_scale(self, scale: Vec2) -> Sprite {
        Sprite {
            vertex_bounds: self.vertex_bounds.scaled_from_origin(scale),
            uv_bounds: self.uv_bounds,
            atlas_index: self.atlas_index,
        }
    }

    pub fn into_vertices(self, pos: WorldPoint, depth: f32, color: Color) -> [Vertex; 4] {
        let vertex = self.vertex_bounds;
        let uv = self.uv_bounds;
        let atlas_index = self.atlas_index as f32;
        let color = color.into();

        [
            Vertex {
                pos: [pos.x + vertex.left, pos.y + vertex.bottom, depth, 1.0],
                uv: [uv.left, uv.bottom, atlas_index],
                color,
            },
            Vertex {
                pos: [pos.x + vertex.right, pos.y + vertex.bottom, depth, 1.0],
                uv: [uv.right, uv.bottom, atlas_index],
                color,
            },
            Vertex {
                pos: [pos.x + vertex.right, pos.y + vertex.top, depth, 1.0],
                uv: [uv.right, uv.top, atlas_index],
                color,
            },
            Vertex {
                pos: [pos.x + vertex.left, pos.y + vertex.top, depth, 1.0],
                uv: [uv.left, uv.top, atlas_index],
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
fn rect_uv_to_line_uv(rect_uv: Rect) -> Line {
    // NOTE: We use only the horizontal axis of a sprite's uv
    Line::new(
        Point::new(rect_uv.left, rect_uv.bottom),
        Point::new(rect_uv.right, rect_uv.top),
    )
}

pub fn vertices_from_rects(
    rect: Rect,
    rect_uv: Rect,
    atlas_index: u32,
    depth: f32,
    color: Color,
) -> [Vertex; 4] {
    let color = color.into();
    let atlas_index = atlas_index as f32;

    [
        Vertex {
            pos: [rect.left, rect.bottom, depth, 1.0],
            uv: [rect_uv.left, rect_uv.bottom, atlas_index],
            color,
        },
        Vertex {
            pos: [rect.right, rect.bottom, depth, 1.0],
            uv: [rect_uv.right, rect_uv.bottom, atlas_index],
            color,
        },
        Vertex {
            pos: [rect.right, rect.top, depth, 1.0],
            uv: [rect_uv.right, rect_uv.top, atlas_index],
            color,
        },
        Vertex {
            pos: [rect.left, rect.top, depth, 1.0],
            uv: [rect_uv.left, rect_uv.top, atlas_index],
            color,
        },
    ]
}
