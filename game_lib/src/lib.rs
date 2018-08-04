pub extern crate cgmath;

pub mod draw;
pub mod math;
pub mod utility;

pub use draw::{DrawCommand, Quad, Vertex, VertexIndex};
pub use math::{Color, Mat4, Point, Rect, SquareMatrix};

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
}

pub fn update_and_draw(input: &GameInput) -> Vec<DrawCommand> {
    let canvas_cursor_pos = Point {
        x: input.cursor_pos_x as f32,
        y: input.cursor_pos_y as f32,
    };
    let canvas_rect = Rect::from_dimension(input.canvas_width as f32, input.canvas_height as f32);

    let mut draw_commands = Vec::new();
    let projection_mat = cgmath::ortho(
        -0.5 * canvas_rect.width,
        0.5 * canvas_rect.width,
        -0.5 * canvas_rect.height,
        0.5 * canvas_rect.height,
        -1.0,
        1.0,
    );

    let mut draw_command = DrawCommand {
        projection: projection_mat.clone().into(),
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: String::from("dummy"),
    };
    let mut another_draw_command = DrawCommand {
        projection: projection_mat.clone().into(),
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: String::from("another_dummy"),
    };

    // Cursor
    let quad_color = Color::new(1.0, 0.0, 0.0, 1.0);
    let dummy_quad = Quad::new(
        Rect::new(
            f32::floor(canvas_cursor_pos.x - 0.5 * canvas_rect.width),
            f32::floor(canvas_cursor_pos.y - 0.5 * canvas_rect.height),
            1.0,
            1.0,
        ),
        -0.1,
        quad_color,
    );
    dummy_quad.append_vertices_indices(
        0,
        &mut another_draw_command.vertices,
        &mut another_draw_command.indices,
    );

    // Dummyquad 1
    let quad_color = Color::new(0.4, 0.7, 0.2, 1.0);
    let dummy_quad = Quad::new(
        Rect::from_dimension(canvas_rect.height, canvas_rect.height),
        -0.7,
        quad_color,
    );
    dummy_quad.append_vertices_indices_centered(
        0,
        &mut draw_command.vertices,
        &mut draw_command.indices,
    );

    // Dummyquad 2
    let quad_color = Color::new(0.9, 0.7, 0.2, 1.0);
    let dummy_quad = Quad::new(
        Rect::from_dimension(canvas_rect.height / 2.0, canvas_rect.height / 2.0),
        -0.2,
        quad_color,
    );
    dummy_quad.append_vertices_indices_centered(
        1,
        &mut another_draw_command.vertices,
        &mut another_draw_command.indices,
    );

    draw_commands.push(draw_command);
    draw_commands.push(another_draw_command);

    draw_commands
}
