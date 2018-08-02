extern crate game_datatypes;
use game_datatypes::{Color, DrawCommand, Point, Quad, Rect};

#[no_mangle]
pub fn update_and_draw(cursor_pos_canvas: Point, canvas_rect: Rect) -> Vec<DrawCommand> {
    let mut draw_commands = Vec::new();

    let projection_mat = game_datatypes::ortho(
        -0.5 * canvas_rect.width,
        0.5 * canvas_rect.width,
        -0.5 * canvas_rect.height,
        0.5 * canvas_rect.height,
        -1.0,
        1.0,
    );

    // Cursor
    let quad_color = Color::new(1.0, 0.0, 0.0, 1.0);
    let dummy_quad = Quad::new(
        Rect::new(
            f32::floor(cursor_pos_canvas.x - 0.5 * canvas_rect.width),
            f32::floor(cursor_pos_canvas.y - 0.5 * canvas_rect.height),
            1.0,
            1.0,
        ),
        -0.1,
        quad_color,
    );
    let mut draw_command = DrawCommand {
        projection: projection_mat.clone().into(),
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: String::from("another_dummy"),
    };
    dummy_quad.append_vertices_indices(0, &mut draw_command.vertices, &mut draw_command.indices);
    draw_commands.push(draw_command);

    // Dummyquad 1
    let quad_color = Color::new(0.4, 0.7, 0.2, 1.0);
    let dummy_quad = Quad::new(
        Rect::from_dimension(canvas_rect.height, canvas_rect.height),
        -0.7,
        quad_color,
    );
    let mut draw_command = DrawCommand {
        projection: projection_mat.clone().into(),
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: String::from("dummy"),
    };
    dummy_quad.append_vertices_indices_centered(
        0,
        &mut draw_command.vertices,
        &mut draw_command.indices,
    );
    draw_commands.push(draw_command);

    // Dummyquad 2
    let quad_color = Color::new(0.9, 0.7, 0.2, 1.0);
    let dummy_quad = Quad::new(
        Rect::from_dimension(canvas_rect.height / 2.0, canvas_rect.height / 2.0),
        -0.2,
        quad_color,
    );
    let mut draw_command = DrawCommand {
        projection: projection_mat.clone().into(),
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: String::from("another_dummy"),
    };
    dummy_quad.append_vertices_indices_centered(
        0,
        &mut draw_command.vertices,
        &mut draw_command.indices,
    );
    draw_commands.push(draw_command);

    draw_commands
}
