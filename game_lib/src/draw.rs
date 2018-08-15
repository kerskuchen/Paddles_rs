use math::{Bounds, Color, Mat4, Point, Rect, Vec2, WorldPoint};

use rgb;
pub use rgb::ComponentBytes;

pub type Pixel = rgb::RGBA8;
pub type VertexIndex = u16;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
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
// DrawCommand
//==================================================================================================
//

#[derive(Debug, Copy, Clone)]
pub enum DrawMode {
    Lines,
    Fill,
}

#[derive(Debug)]
pub enum FramebufferTarget {
    Screen,
    Offscreen(FramebufferInfo),
}

#[derive(Debug)]
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

impl DrawCommand {
    pub fn from_quads(
        transform: Mat4,
        texture_info: TextureInfo,
        framebuffer: FramebufferTarget,
        batch: QuadBatch,
    ) -> DrawCommand {
        let (vertices, indices) = batch.into_vertices_indices();
        DrawCommand::Draw {
            transform,
            vertices,
            indices,
            texture_info,
            framebuffer,
            draw_mode: DrawMode::Fill,
        }
    }

    pub fn from_lines(
        transform: Mat4,
        texture_info: TextureInfo,
        framebuffer: FramebufferTarget,
        batch: LineBatch,
    ) -> DrawCommand {
        let (vertices, indices) = batch.into_vertices_indices();
        DrawCommand::Draw {
            transform,
            vertices,
            indices,
            texture_info,
            framebuffer,
            draw_mode: DrawMode::Lines,
        }
    }
}

//==================================================================================================
// Batch drawing
//==================================================================================================
//
pub struct QuadBatch {
    vertices: Vec<Vertex>,
}

impl QuadBatch {
    const VERTICES_PER_QUAD: usize = 4;
    const INDICES_PER_QUAD: usize = 6;

    pub fn new() -> QuadBatch {
        QuadBatch {
            vertices: Vec::new(),
        }
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.vertices.extend_from_slice(&quad.into_vertices());
    }

    pub fn push_quad_centered(&mut self, quad: Quad) {
        self.vertices
            .extend_from_slice(&quad.into_vertices_centered());
    }

    pub fn push_sprite(&mut self, sprite: Sprite, pos: WorldPoint, depth: f32, color: Color) {
        self.vertices
            .extend_from_slice(&sprite.into_vertices(pos, depth, color));
    }

    pub fn into_vertices_indices(self) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let num_quads = self.vertices.len() / QuadBatch::VERTICES_PER_QUAD;
        let num_indices = num_quads * QuadBatch::INDICES_PER_QUAD;

        let mut indices = Vec::with_capacity(num_indices);
        for quad_index in 0..(num_indices as VertexIndex) {
            let quad_indices = [
                4 * quad_index,
                4 * quad_index + 1,
                4 * quad_index + 2,
                4 * quad_index + 2,
                4 * quad_index + 3,
                4 * quad_index,
            ];
            indices.extend(&quad_indices);
        }
        (self.vertices, indices)
    }
}

pub struct LineBatch {
    vertices: Vec<Vertex>,
}

impl LineBatch {
    const VERTICES_PER_LINE: usize = 2;
    const INDICES_PER_LINE: usize = 2;

    pub fn new() -> LineBatch {
        LineBatch {
            vertices: Vec::new(),
        }
    }

    pub fn push_line(&mut self, start: Point, end: Point, depth: f32, color: Color) {
        let color = color.into();

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let line_vertices = [
            Vertex {
                pos: [start.x, start.y, depth, 1.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                pos: [end.x, end.y, depth, 1.0],
                uv: [1.0, 0.0],
                color,
            },
        ];
        self.vertices.extend_from_slice(&line_vertices);
    }

    pub fn into_vertices_indices(self) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let num_lines = self.vertices.len() / LineBatch::VERTICES_PER_LINE;
        let num_indices = num_lines * LineBatch::INDICES_PER_LINE;

        let mut indices = Vec::with_capacity(num_indices);
        for line_index in 0..(num_indices as VertexIndex) {
            let line_indices = [2 * line_index, 2 * line_index + 1];
            indices.extend(&line_indices);
        }
        (self.vertices, indices)
    }
}

//==================================================================================================
// Sprite
//==================================================================================================
//

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
#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub rect: Rect,
    pub depth: f32,
    pub color: Color,
}

impl Quad {
    pub fn new(rect: Rect, depth: f32, color: Color) -> Quad {
        Quad { rect, depth, color }
    }

    pub fn into_vertices(self) -> [Vertex; 4] {
        let bounds = self.rect.to_bounds();
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

    pub fn into_vertices_centered(self) -> [Vertex; 4] {
        let bounds = self.rect.to_bounds_centered();
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
