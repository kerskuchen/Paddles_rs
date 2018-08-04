pub extern crate cgmath;

pub mod draw;
pub mod math;
#[macro_use]
pub mod utility;

pub use draw::{DrawBatch, DrawCommand, DrawMode, Quad, Vertex, VertexIndex};
pub use math::{Camera, Color, Mat4, Mat4Helper, Point, Rect, SquareMatrix, Vec2};

pub struct GameInput {
    pub canvas_width: i32,
    pub canvas_height: i32,

    // NOTE: The cursor position is given in the following interval:
    //       [0 .. canvas_width - 1] x [0 .. canvas_height - 1]
    //       where (0,0) is the bottom left of the screen.
    pub cursor_pos_x: i32,
    pub cursor_pos_y: i32,
}

impl GameInput {
    pub fn new() -> GameInput {
        GameInput {
            canvas_width: 0,
            canvas_height: 0,
            cursor_pos_x: 0,
            cursor_pos_y: 0,
        }
    }

    /// Returns the current cursor position in normalized screen coordinates which are given as the
    /// following half open interval:
    ///
    /// `[0 .. 1[ x [0 .. 1[`
    /// where `(0,0)` represents the bottom left of the screen.
    pub fn relative_cursor_pos(&self) -> Point {
        Point::new(
            self.cursor_pos_x as f32 / self.canvas_width as f32,
            self.cursor_pos_y as f32 / self.canvas_height as f32,
        )
    }
}

pub fn update_and_draw(input: &GameInput) -> Vec<DrawCommand> {
    let canvas_cursor_pos = Point {
        x: input.cursor_pos_x as f32,
        y: input.cursor_pos_y as f32,
    };
    let canvas_rect =
        Rect::from_width_height(input.canvas_width as f32, input.canvas_height as f32);

    // TODO(JaSc): Fix and standardize z_near/z_far
    let cam = Camera::new(input.canvas_width, input.canvas_height, -1.0, 1.0);
    let normalized_screen_cursor_pos = canvas_cursor_pos / canvas_rect.dim;
    let world_cursor_pos = cam.screen_to_world(normalized_screen_cursor_pos);

    // ---------------------------------------------------------------------------------------------
    // Generate vertices
    //
    let mut plain_batch = DrawBatch::new(DrawMode::Quads);
    let mut textured_batch = DrawBatch::new(DrawMode::Quads);

    // Cursor
    let cursor_color = Color::new(1.0, 0.0, 0.0, 1.0);
    let cursor_quad = Quad::new(
        Rect::from_point(world_cursor_pos, math::PIXEL_SIZE, math::PIXEL_SIZE),
        -0.1,
        cursor_color,
    );
    plain_batch.push_quad(cursor_quad);

    // Grid
    let grid_dark = Color::new(0.5, 0.3, 0.0, 1.0);
    let grid_light = Color::new(0.9, 0.7, 0.2, 1.0);
    let rect_dim = Vec2::new(1.0, 1.0);
    for x in -10..10 {
        for dia in -10..10 {
            let pos = Point::new((x + dia) as f32, dia as f32);
            if x % 2 == 0 {
                textured_batch.push_quad(Quad::new(
                    Rect::from_point_dimension(pos, rect_dim),
                    -0.2,
                    grid_dark,
                ));
            } else {
                plain_batch.push_quad(Quad::new(
                    Rect::from_point_dimension(pos, rect_dim),
                    -0.2,
                    grid_light,
                ));
            }
        }
    }

    vec![
        DrawCommand::new(cam.proj_view_matrix(), "dummy", textured_batch),
        DrawCommand::new(cam.proj_view_matrix(), "another_dummy", plain_batch),
    ]
}
