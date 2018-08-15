#![feature(nll)]

pub extern crate cgmath;
extern crate lodepng;
extern crate rgb;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;
extern crate bincode;

use std::collections::HashMap;
use std::fs::File;

pub mod draw;
pub mod math;
#[macro_use]
pub mod utility;

// TODO(JaSc): We need more consistency in names: Is it FrameBuffer or Framebuffer?
//             Is it GameState or Gamestate? When its GameState why do variables are then called
//             gamestate and not game_state?
pub use draw::{
    ComponentBytes, DrawCommand, DrawMode, FramebufferInfo, FramebufferTarget, LineBatch, Pixel,
    Quad, QuadBatch, Sprite, TextureInfo, Vertex, VertexIndex,
};
pub use math::{
    Bounds, Camera, Color, Mat4, Mat4Helper, Point, Rect, ScreenPoint, SquareMatrix, Vec2,
    WorldPoint, PIXELS_PER_UNIT, PIXEL_SIZE,
};

const CANVAS_WIDTH: i32 = 480;
const CANVAS_HEIGHT: i32 = 270;

pub struct GameState {
    is_initialized: bool,
    screen_dim: Vec2,
    canvas_framebuffer: FramebufferInfo,
    texture_atlas: TextureInfo,
    texture_font: TextureInfo,
    sprite_map: HashMap<String, Sprite>,
    glyph_sprites: Vec<Sprite>,
    mouse_pos_screen: ScreenPoint,
    mouse_pos_world: WorldPoint,
    cam: Camera,
}

impl GameState {
    fn new() -> GameState {
        GameState {
            is_initialized: false,
            screen_dim: Vec2::zero(),
            canvas_framebuffer: FramebufferInfo::empty(),
            texture_atlas: TextureInfo::empty(),
            texture_font: TextureInfo::empty(),
            sprite_map: HashMap::new(),
            glyph_sprites: Vec::new(),
            // TODO(JaSc): Fix and standardize z_near/z_far
            cam: Camera::new(CANVAS_WIDTH, CANVAS_HEIGHT, -1.0, 1.0),
            mouse_pos_screen: ScreenPoint::zero(),
            mouse_pos_world: WorldPoint::zero(),
        }
    }
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
    pub screen_dim: Vec2,

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
            screen_dim: Vec2::zero(),

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

pub fn create_gamestate() -> GameState {
    GameState::new()
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

fn initialize_gamestate(gamestate: &mut GameState) -> Vec<DrawCommand> {
    let mut draw_commands = Vec::new();

    // Load atlas texture and sprites
    let (texture_info, pixels) = load_texture(0, "data/images/atlas.png");
    gamestate.texture_atlas = texture_info.clone();
    draw_commands.push(DrawCommand::CreateTexture {
        texture_info,
        pixels,
    });

    let mut atlas_metafile =
        File::open("data/images/atlas.tex").expect("Could not load atlas metafile");
    gamestate.sprite_map =
        bincode::deserialize_from(&mut atlas_metafile).expect("Could not deserialize sprite map");

    // Load font texture and sprites
    let (texture_info, pixels) = load_texture(1, "data/fonts/04B_03__.png");
    gamestate.texture_font = texture_info.clone();
    draw_commands.push(DrawCommand::CreateTexture {
        texture_info,
        pixels,
    });

    let mut font_metafile =
        File::open("data/fonts/04B_03__.fnt").expect("Could not load font metafile");
    gamestate.glyph_sprites =
        bincode::deserialize_from(&mut font_metafile).expect("Could not deserialize font glyphs");

    // TODO(JaSc): Create new framebuffer

    draw_commands
}

pub fn update_and_draw(input: &GameInput, mut gamestate: &mut GameState) -> Vec<DrawCommand> {
    // TODO(JaSc): Maybe we additionally want something like SystemCommands that tell the platform
    //             layer to create framebuffers / go fullscreen / turn on vsync / upload textures
    let mut draw_commands = Vec::new();

    // TODO(JaSc): Change letterbox color based on debug/release
    let letterbox_color = Color::new(1.0, 0.4, 0.7, 1.0);
    draw_commands.push(DrawCommand::Clear {
        framebuffer: FramebufferTarget::Screen,
        color: letterbox_color,
    });

    // Load sprites if needed
    if !gamestate.is_initialized {
        gamestate.is_initialized = true;
        draw_commands.append(&mut initialize_gamestate(&mut gamestate));
    }

    if gamestate.screen_dim != input.screen_dim {
        gamestate.screen_dim = input.screen_dim;
        let screen_rect = Rect::from_dimension(gamestate.screen_dim);

        info!("=====================");
        info!(
            "Window resized: {} x {}",
            screen_rect.width() as i32,
            screen_rect.height() as i32
        );
        info!("Canvas size: {} x {}", CANVAS_WIDTH, CANVAS_HEIGHT);

        // TODO(JaSc): Calculate new blit rect
        // TODO(JaSc): Create new framebuffer
        // TODO(JaSc): Delete old framebuffer

        // info!("Blit-rect: {:?}", rc.canvas_blit_rect());
        // info!(
        //     "Pixel scale factor: {} ",
        //     if rc.canvas_blit_rect().pos.x == 0.0 {
        //         rc.screen_rect().width() / rc.canvas_rect().width()
        //     } else {
        //         rc.screen_rect().height() / rc.canvas_rect().height()
        //     }
        // );
        // info!(
        //     "Pixel waste: {} x {}",
        //     rc.screen_rect().width() - rc.canvas_blit_rect().width(),
        //     rc.screen_rect().height() - rc.canvas_blit_rect().height(),
        // );
        // info!("=====================");
    }

    // TODO(JaSc): Re-evaluate the need for the +(0.5, 0.5) offset
    //
    // NOTE: We add (0.5, 0,5) to the cursors' pixel-position as we want the cursor to be in the
    //       center of the canvas' pixel. This prevents artifacts when pixel-snapping the
    //       cursor world-position later.
    // Example:
    // If we transform canvas cursor pixel-position (2,0) to its world position and back to its
    // canvas pixel-position we get (1.9999981, 0.0). If we would pixel-snap this coordinate
    // (effectively flooring it), we would get (1.0, 0.0) which would be wrong.
    // Adding 0.5 gives us a correct flooring result.
    //
    let screen_rect = Rect::from_dimension(gamestate.screen_dim);
    let canvas_rect = Rect::from_width_height(CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32);
    let canvas_cursor_pos =
        screen_coord_to_canvas_coord(input.mouse_pos_screen, screen_rect, canvas_rect)
            + Vec2::new(0.5, 0.5);

    // Screen mouse position
    let new_mouse_pos_screen = input.mouse_pos_screen;
    let mouse_delta_screen = new_mouse_pos_screen - gamestate.mouse_pos_screen;
    gamestate.mouse_pos_screen = new_mouse_pos_screen;

    // World mouse position
    let new_mouse_pos_world = gamestate.cam.screen_to_world(new_mouse_pos_screen);
    let _mouse_delta_world = new_mouse_pos_world - gamestate.mouse_pos_world;
    gamestate.mouse_pos_world = new_mouse_pos_world;

    if input.mouse_button_right.is_pressed {
        gamestate.cam.pan(mouse_delta_screen);
    }

    if input.mouse_button_middle.is_pressed {
        gamestate.cam.zoom_to_world_point(new_mouse_pos_world, 1.0);
    }

    if input.mouse_wheel_delta > 0 {
        let new_zoom_level = f32::min(gamestate.cam.zoom_level * 2.0, 8.0);
        gamestate
            .cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    } else if input.mouse_wheel_delta < 0 {
        let new_zoom_level = f32::max(gamestate.cam.zoom_level / 2.0, 1.0 / 32.0);
        gamestate
            .cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    }

    // ---------------------------------------------------------------------------------------------
    // Generate draw commands
    //

    let mut line_batch = LineBatch::new();
    let mut fill_batch = QuadBatch::new();

    // Draw line from origin to cursor position
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
    fill_batch.push_sprite(
        gamestate.sprite_map["images/plain"].with_scale(Vec2::new(PIXEL_SIZE, PIXEL_SIZE)),
        new_mouse_pos_world.pixel_snapped(),
        -0.1,
        cursor_color,
    );

    // Grid
    let grid_dark = Color::new(0.5, 0.3, 0.0, 1.0);
    let grid_light = Color::new(0.9, 0.7, 0.2, 1.0);
    for x in -10..10 {
        for diagonal in -10..10 {
            let pos = Point::new((x + diagonal) as f32, diagonal as f32);
            if x % 2 == 0 {
                fill_batch.push_sprite(
                    gamestate.sprite_map["images/textured"],
                    pos,
                    -0.2,
                    grid_dark,
                );
            } else {
                fill_batch.push_sprite(gamestate.sprite_map["images/plain"], pos, -0.2, grid_light);
            }
        }
    }

    let transform = gamestate.cam.proj_view_matrix();
    draw_commands.push(DrawCommand::from_lines(
        transform,
        gamestate.texture_atlas.clone(),
        FramebufferTarget::Offscreen(gamestate.canvas_framebuffer.clone()),
        line_batch,
    ));
    draw_commands.push(DrawCommand::from_quads(
        transform,
        gamestate.texture_atlas.clone(),
        FramebufferTarget::Offscreen(gamestate.canvas_framebuffer.clone()),
        fill_batch,
    ));

    draw_commands.push(DrawCommand::BlitFramebuffer {
        source_framebuffer: gamestate.canvas_framebuffer.clone(),
        target_framebuffer: FramebufferTarget::Screen,
        source_rect: canvas_rect,
        target_rect: canvas_blit_rect(screen_rect, canvas_rect),
    });

    draw_commands
}

// TODO(JaSc): Proofread and refactor this
/// Returns the dimensions of the `blit_rectangle` of the canvas in pixels.
/// The `blit-rectange` is the area of the screen where the content of the canvas is drawn onto.
/// It is as big as a canvas that is proportionally stretched and centered to fill the whole
/// screen.
///
/// It may or may not be smaller than the full screen size depending on the aspect
/// ratio of both the screen and the canvas. The `blit_rectange` is guaranteed to either have
/// the same width a as the screen (with letterboxing if needed) or the same height as the
/// screen (with columnboxing if needed) or completely fill the screen.
pub fn canvas_blit_rect(screen_rect: Rect, canvas_rect: Rect) -> Rect {
    canvas_rect
        .stretched_to_fit(screen_rect)
        .centered_in(screen_rect)
}

// TODO(JaSc): Proofread and refactor this
/// Clamps a given `screen_point` to the area of the
/// [`canvas_blit_rect`](#method.canvas_blit_rect) and converts the result into
/// a canvas-position in the following interval:
/// `[0..canvas_rect.width-1]x[0..canvas_rect.height-1]`
/// where `(0,0)` is the bottom left of the canvas.
fn screen_coord_to_canvas_coord(
    screen_point: Point,
    screen_rect: Rect,
    canvas_rect: Rect,
) -> Point {
    // NOTE: Clamping the point needs to use integer arithmetic such that
    //          x != canvas.rect.width and y != canvas.rect.height
    //       holds. We therefore need to subtract one from the blit_rect's dimension and then
    //       add one again after clamping to achieve the desired effect.
    // TODO(JaSc): Maybe make this more self documenting via integer rectangles
    let mut blit_rect = canvas_blit_rect(screen_rect, canvas_rect);
    blit_rect.dim -= 1.0;
    let clamped_point = screen_point.clamped_in_rect(blit_rect);
    blit_rect.dim += 1.0;

    let result = canvas_rect.dim * ((clamped_point - blit_rect.pos) / blit_rect.dim);
    Point::new(f32::floor(result.x), f32::floor(result.y))
}
