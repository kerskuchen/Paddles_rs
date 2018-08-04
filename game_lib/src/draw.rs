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
    pub fn new(transform: Mat4, texture_name: &str, batch: DrawBatch) -> DrawCommand {
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

pub enum DrawMode {
    Lines,
    Quads,
}

pub struct DrawBatch {
    vertices_per_elem: VertexIndex,
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
}

impl DrawBatch {
    pub fn new(draw_mode: DrawMode) -> DrawBatch {
        DrawBatch {
            vertices_per_elem: match draw_mode {
                DrawMode::Lines => 2,
                DrawMode::Quads => 4,
            },
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn push_quad(&mut self, quad: Quad) {
        let quad_num = self.vertices.len() as VertexIndex / self.vertices_per_elem;
        let (self_vertices, self_indices) = quad.into_vertices_indices(quad_num);

        self.vertices.extend_from_slice(&self_vertices);
        self.indices.extend(&self_indices);
    }

    pub fn push_quad_centered(&mut self, quad: Quad) {
        let quad_num = self.vertices.len() as VertexIndex / self.vertices_per_elem;
        let (self_vertices, self_indices) = quad.into_vertices_indices_centered(quad_num);

        self.vertices.extend_from_slice(&self_vertices);
        self.indices.extend(&self_indices);
    }

    pub fn into_vertices_indices(self) -> (Vec<Vertex>, Vec<VertexIndex>) {
        (self.vertices, self.indices)
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

    pub fn unit_quad(depth: f32, color: Color) -> Quad {
        Quad {
            rect: Rect::from_width_height(1.0, 1.0),
            depth,
            color,
        }
    }

    pub fn into_vertices_indices(self, quad_index: VertexIndex) -> ([Vertex; 4], [VertexIndex; 6]) {
        let bounds = self.rect.bounds();
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let vertices: [Vertex; 4] = [
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
        ];

        let indices: [VertexIndex; 6] = [
            4 * quad_index,
            4 * quad_index + 1,
            4 * quad_index + 2,
            4 * quad_index + 2,
            4 * quad_index + 3,
            4 * quad_index,
        ];

        (vertices, indices)
    }

    pub fn into_vertices_indices_centered(
        self,
        quad_index: VertexIndex,
    ) -> ([Vertex; 4], [VertexIndex; 6]) {
        let bounds = self.rect.bounds_centered();
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let vertices: [Vertex; 4] = [
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
        ];

        let indices: [VertexIndex; 6] = [
            4 * quad_index,
            4 * quad_index + 1,
            4 * quad_index + 2,
            4 * quad_index + 2,
            4 * quad_index + 3,
            4 * quad_index,
        ];

        (vertices, indices)
    }
}
