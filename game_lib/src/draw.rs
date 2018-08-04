use math::{Color, Mat4, Rect};

pub type VertexIndex = u16;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

//==================================================================================================
// DrawCommand
//==================================================================================================
//

pub struct DrawCommand {
    pub transform: Mat4,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<VertexIndex>,
    pub texture: String,
}

impl DrawCommand {
    pub fn new(transform: Mat4, texture_name: &str, batch: QuadBatch) -> DrawCommand {
        let (vertices, indices) = batch.into_vertices_indices();
        DrawCommand {
            transform,
            vertices,
            indices,
            texture: String::from(texture_name),
        }
    }
}

//==================================================================================================
// DrawBatch
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

    pub fn into_vertices_indices(self) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let num_quads = self.vertices.len() / QuadBatch::VERTICES_PER_QUAD;
        let num_indices = num_quads * QuadBatch::INDICES_PER_QUAD;

        let mut indices = Vec::with_capacity(num_indices);
        for quad_index in 0..(num_indices as VertexIndex) {
            let quad_indices: [VertexIndex; 6] = [
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
        let bounds = self.rect.bounds();
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
        let bounds = self.rect.bounds_centered();
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
