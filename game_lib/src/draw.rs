use math::{Color, Mat4, Rect};

pub struct DrawCommand {
    pub projection: Mat4,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<VertexIndex>,
    pub texture: String,
}

pub type VertexIndex = u16;
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
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

    // TODO(JaSc): Create vertex/index-buffer struct and move the `append_..` methods into that
    pub fn append_vertices_indices(
        &self,
        quad_index: VertexIndex,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<VertexIndex>,
    ) {
        let (self_vertices, self_indices) = self.into_vertices_indices(quad_index);
        vertices.extend_from_slice(&self_vertices);
        indices.extend(&self_indices);
    }

    pub fn append_vertices_indices_centered(
        &self,
        quad_index: VertexIndex,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<VertexIndex>,
    ) {
        let (self_vertices, self_indices) = self.into_vertices_indices_centered(quad_index);
        vertices.extend_from_slice(&self_vertices);
        indices.extend(&self_indices);
    }

    pub fn into_vertices_indices(self, quad_index: VertexIndex) -> ([Vertex; 4], [VertexIndex; 6]) {
        let pos = self.rect.pos;
        let dim = self.rect.dim;
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let vertices: [Vertex; 4] = [
            Vertex {
                pos: [pos.x, pos.y, depth, 1.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + dim.x, pos.y, depth, 1.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + dim.x, pos.y + dim.y, depth, 1.0],
                uv: [1.0, 0.0],
                color,
            },
            Vertex {
                pos: [pos.x, pos.y + dim.y, depth, 1.0],
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
        let pos = self.rect.pos;
        let half_dim = 0.5 * self.rect.dim;
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let vertices: [Vertex; 4] = [
            Vertex {
                pos: [pos.x - half_dim.x, pos.y - half_dim.y, depth, 1.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + half_dim.x, pos.y - half_dim.y, depth, 1.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + half_dim.x, pos.y + half_dim.y, depth, 1.0],
                uv: [1.0, 0.0],
                color,
            },
            Vertex {
                pos: [pos.x - half_dim.x, pos.y + half_dim.y, depth, 1.0],
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
