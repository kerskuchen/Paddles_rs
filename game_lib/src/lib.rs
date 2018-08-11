pub extern crate cgmath;
extern crate lodepng;
extern crate rgb;

#[macro_use]
extern crate serde_derive;
extern crate bincode;

use std::fs::File;

pub mod draw;
pub mod math;
#[macro_use]
pub mod utility;

pub use draw::{
    Bounds, ComponentBytes, DrawCommand, FontHeader, LineBatch, Pixel, Quad, QuadBatch, Sprite,
    Texture, Vertex, VertexIndex,
};
pub use math::{
    Camera, Color, Mat4, Mat4Helper, Point, Rect, ScreenPoint, SquareMatrix, Vec2, WorldPoint,
};

pub struct GameState {
    loaded_resources: bool,
    texture_atlas: Texture,
    sprites: Vec<Sprite>,
    texture_font: Texture,
    mouse_pos_screen: ScreenPoint,
    mouse_pos_world: WorldPoint,
    cam: Camera,
}

pub struct GameButton {
    pub num_state_transitions: u32,
    pub is_pressed: bool,
}

impl GameButton {
    pub fn new() -> GameButton {
        GameButton {
            num_state_transitions: 0,
            is_pressed: false,
        }
    }

    pub fn set_state(&mut self, is_pressed: bool) {
        if self.is_pressed != is_pressed {
            self.num_state_transitions += 1;
            self.is_pressed = is_pressed;
        }
    }

    pub fn clear_transitions(&mut self) {
        self.num_state_transitions = 0;
    }
}

pub struct GameInput {
    pub mouse_button_left: GameButton,
    pub mouse_button_middle: GameButton,
    pub mouse_button_right: GameButton,
    pub mouse_pos_screen: ScreenPoint,

    /// * `Positive`: Moving away from user
    /// * `Negative`: Moving towards from user
    pub mouse_wheel_delta: i32,
}

impl GameInput {
    pub fn new() -> GameInput {
        GameInput {
            mouse_button_left: GameButton::new(),
            mouse_button_middle: GameButton::new(),
            mouse_button_right: GameButton::new(),
            mouse_pos_screen: ScreenPoint::zero(),
            mouse_wheel_delta: 0,
        }
    }

    pub fn clear_button_transitions(&mut self) {
        self.mouse_button_left.clear_transitions();
        self.mouse_button_middle.clear_transitions();
        self.mouse_button_right.clear_transitions();
        self.mouse_wheel_delta = 0;
    }
}

pub fn initialize(canvas_width: i32, canvas_height: i32) -> GameState {
    GameState {
        loaded_resources: false,
        texture_atlas: Texture::empty(),
        sprites: Vec::new(),
        texture_font: Texture::empty(),
        // TODO(JaSc): Fix and standardize z_near/z_far
        cam: Camera::new(canvas_width, canvas_height, -1.0, 1.0),
        mouse_pos_screen: ScreenPoint::zero(),
        mouse_pos_world: WorldPoint::zero(),
    }
}

fn load_texture(id: u32, file_name: &str) -> (Texture, Vec<rgb::RGBA8>) {
    let img = lodepng::decode32_file(file_name).unwrap();
    let info = Texture {
        id,
        width: img.width as u16,
        height: img.height as u16,
        name: String::from(file_name),
    };
    (info, img.buffer)
}

pub fn update_and_draw(input: &GameInput, game_state: &mut GameState) -> Vec<DrawCommand> {
    // Screen mouse position
    let new_mouse_pos_screen = input.mouse_pos_screen;
    let mouse_delta_screen = new_mouse_pos_screen - game_state.mouse_pos_screen;
    game_state.mouse_pos_screen = new_mouse_pos_screen;

    // World mouse position
    let new_mouse_pos_world = game_state.cam.screen_to_world(new_mouse_pos_screen);
    let _mouse_delta_world = new_mouse_pos_world - game_state.mouse_pos_world;
    game_state.mouse_pos_world = new_mouse_pos_world;

    if input.mouse_button_right.is_pressed {
        game_state.cam.pan(mouse_delta_screen);
    }

    if input.mouse_button_middle.is_pressed {
        game_state.cam.zoom_to_world_point(new_mouse_pos_world, 1.0);
    }

    if input.mouse_wheel_delta > 0 {
        let new_zoom_level = f32::min(game_state.cam.zoom_level * 2.0, 8.0);
        game_state
            .cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    } else if input.mouse_wheel_delta < 0 {
        let new_zoom_level = f32::max(game_state.cam.zoom_level / 2.0, 1.0 / 32.0);
        game_state
            .cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    }

    // ---------------------------------------------------------------------------------------------
    // Generate quads
    //

    let mut draw_commands = Vec::new();

    if !game_state.loaded_resources {
        game_state.loaded_resources = true;

        // Load atlas texture and sprites
        let (texture, pixels) = load_texture(0, "data/images/atlas.png");
        game_state.texture_atlas = texture.clone();
        draw_commands.push(DrawCommand::UploadTexture { texture, pixels });

        let mut atlas_metafile =
            File::open("data/images/atlas.tex").expect("Could not load atlas file");
        let sprite_mapping: Vec<(usize, String)> = bincode::deserialize_from(&mut atlas_metafile)
            .expect("Could not deserialize sprite mapping");
        let sprites: Vec<Sprite> =
            bincode::deserialize_from(&mut atlas_metafile).expect("Could not deserialize sprites");

        dprintln!(sprite_mapping);
        dprintln!(sprites);

        // Load font
        let (texture, pixels) = load_texture(1, "data/fonts/04B_03__.png");
        game_state.texture_font = texture.clone();
        draw_commands.push(DrawCommand::UploadTexture { texture, pixels });
    }

    let mut line_batch = LineBatch::new();
    let mut plain_batch = QuadBatch::new();
    let mut textured_batch = QuadBatch::new();

    let line_start = WorldPoint::new(0.0, 0.0);
    let line_end = new_mouse_pos_world;
    line_batch.push_line(line_start, line_end, 0.0, Color::new(1.0, 0.0, 0.0, 1.0));

    // Cursor
    let mut cursor_color = Color::new(0.0, 0.0, 0.0, 1.0);
    if input.mouse_button_left.is_pressed {
        cursor_color.x = 1.0;
    }
    if input.mouse_button_middle.is_pressed {
        cursor_color.y = 1.0;
    }
    if input.mouse_button_right.is_pressed {
        cursor_color.z = 1.0;
    }

    let cursor_quad = Quad::new(
        Rect::from_point(new_mouse_pos_world, math::PIXEL_SIZE, math::PIXEL_SIZE),
        -0.1,
        cursor_color,
    );
    plain_batch.push_quad(cursor_quad);
    let cursor_quad = Quad::new(
        Rect::from_point(
            new_mouse_pos_world.pixel_snapped(),
            math::PIXEL_SIZE,
            math::PIXEL_SIZE,
        ),
        -0.1,
        Color::new(1.0, 1.0, 1.0, 1.0),
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

    let transform = game_state.cam.proj_view_matrix();
    draw_commands.push(DrawCommand::from_quads(
        transform,
        game_state.texture_atlas.clone(),
        textured_batch,
    ));
    draw_commands.push(DrawCommand::from_quads(
        transform,
        game_state.texture_atlas.clone(),
        plain_batch,
    ));
    draw_commands.push(DrawCommand::from_lines(
        transform,
        game_state.texture_atlas.clone(),
        line_batch,
    ));

    draw_commands
}
