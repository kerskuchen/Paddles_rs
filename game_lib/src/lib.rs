#![feature(nll)]

pub extern crate cgmath;
extern crate lodepng;
extern crate rgb;

#[macro_use]
extern crate log;
extern crate fern;

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
    Bounds, Camera, CanvasPoint, Color, Mat4, Mat4Helper, Point, Rect, SquareMatrix, Vec2,
    WorldPoint,
};

const UNIT_SIZE: f32 = 16.0;
const CANVAS_WIDTH: i32 = 480;
const CANVAS_HEIGHT: i32 = 270;

const LOG_LEVEL_GENERAL: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_GAME_LIB: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_MATH: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_DRAW: log::LevelFilter = log::LevelFilter::Trace;

pub struct GameState {
    screen_dim: Vec2,
    canvas_framebuffer: Option<FramebufferInfo>,

    texture_atlas: Option<TextureInfo>,
    texture_font: Option<TextureInfo>,
    sprite_map: HashMap<String, Sprite>,
    glyph_sprites: Vec<Sprite>,

    mouse_pos_canvas: CanvasPoint,
    mouse_pos_world: WorldPoint,

    origin: WorldPoint,
    cam: Camera,
}

impl GameState {
    fn new() -> GameState {
        GameState {
            screen_dim: Vec2::zero(),
            canvas_framebuffer: None,

            texture_atlas: None,
            texture_font: None,
            sprite_map: HashMap::new(),
            glyph_sprites: Vec::new(),

            mouse_pos_canvas: CanvasPoint::zero(),
            mouse_pos_world: WorldPoint::zero(),

            // TODO(JaSc): Fix and standardize z_near/z_far
            cam: Camera::new(CANVAS_WIDTH, CANVAS_HEIGHT, -1.0, 1.0),
            origin: WorldPoint::zero(),
        }
    }
}

//==================================================================================================
// GameInput
//==================================================================================================
//
#[derive(Default)]
pub struct GameInput {
    pub time_since_startup: f64,
    pub time_delta: f32,
    pub time_update: f32,
    pub time_draw: f32,

    pub screen_dim: Vec2,

    pub do_reinit_gamestate: bool,
    pub do_reinit_drawstate: bool,
    pub hotreload_happened: bool,
    pub direct_screen_drawing: bool,

    /// Mouse position is given in the following interval:
    /// [0 .. screen_width - 1] x [0 .. screen_height - 1]
    /// where (0,0) is the bottom left of the screen
    pub mouse_pos_screen: CanvasPoint,

    pub mouse_button_left: GameButton,
    pub mouse_button_middle: GameButton,
    pub mouse_button_right: GameButton,

    /// * `Positive`: Moving away from user
    /// * `Negative`: Moving towards user
    pub mouse_wheel_delta: i32,
}

impl GameInput {
    pub fn new() -> GameInput {
        GameInput {
            time_since_startup: 0.0,
            time_delta: 0.0,
            time_update: 0.0,
            time_draw: 0.0,

            screen_dim: Vec2::zero(),

            do_reinit_gamestate: true,
            do_reinit_drawstate: true,
            hotreload_happened: true,
            direct_screen_drawing: false,

            mouse_pos_screen: CanvasPoint::zero(),
            mouse_button_left: GameButton::new(),
            mouse_button_middle: GameButton::new(),
            mouse_button_right: GameButton::new(),
            mouse_wheel_delta: 0,
        }
    }

    pub fn clear_flags(&mut self) {
        self.do_reinit_gamestate = false;
        self.do_reinit_drawstate = false;
        self.hotreload_happened = false;
    }

    pub fn clear_button_transitions(&mut self) {
        self.mouse_button_left.clear_transitions();
        self.mouse_button_middle.clear_transitions();
        self.mouse_button_right.clear_transitions();
        self.mouse_wheel_delta = 0;
    }
}

#[derive(Default)]
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

    pub fn toggle(&mut self) {
        if self.is_pressed {
            self.set_state(false);
        } else {
            self.set_state(true);
        }
    }
}

//==================================================================================================
// Game
//==================================================================================================
//
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

fn reinitialize_after_hotreload() {
    // Initializing logger
    // NOTE: When hot reloading the game lib dll the logging must be reinitialized
    //
    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{}: {}", record.level(), message)))
        .level(LOG_LEVEL_GENERAL)
        .level_for("game_lib", LOG_LEVEL_GAME_LIB)
        .level_for("game_lib::math", LOG_LEVEL_MATH)
        .level_for("game_lib::draw", LOG_LEVEL_DRAW)
        .chain(std::io::stdout())
        .apply()
        .expect("Could not initialize logger");
}

fn reinitialize_gamestate(gamestate: &mut GameState) {
    gamestate.origin = WorldPoint::new((1 << 18) as f32, -(1 << 18) as f32);
    gamestate.cam = Camera::with_position(gamestate.origin, CANVAS_WIDTH, CANVAS_HEIGHT, -1.0, 1.0);
}

fn reinitialize_drawstate(gamestate: &mut GameState) -> Vec<DrawCommand> {
    let mut draw_commands = Vec::new();
    if gamestate.texture_atlas.is_none() {
        // Load atlas texture and sprites
        let (texture_info, pixels) = load_texture(0, "data/images/atlas.png");
        gamestate.texture_atlas = Some(texture_info.clone());
        draw_commands.push(DrawCommand::CreateTexture {
            texture_info,
            pixels,
        });
        let mut atlas_metafile =
            File::open("data/images/atlas.tex").expect("Could not load atlas metafile");
        gamestate.sprite_map = bincode::deserialize_from(&mut atlas_metafile)
            .expect("Could not deserialize sprite map");
    }

    if gamestate.texture_font.is_none() {
        // Load font texture and sprites
        let (texture_info, pixels) = load_texture(1, "data/fonts/04B_03__.png");
        gamestate.texture_font = Some(texture_info.clone());
        draw_commands.push(DrawCommand::CreateTexture {
            texture_info,
            pixels,
        });
        let mut font_metafile =
            File::open("data/fonts/04B_03__.fnt").expect("Could not load font metafile");
        gamestate.glyph_sprites = bincode::deserialize_from(&mut font_metafile)
            .expect("Could not deserialize font glyphs");
    }

    if gamestate.canvas_framebuffer.is_none() {
        // Create new canvas framebuffer
        let framebuffer_info = FramebufferInfo {
            id: 0,
            width: CANVAS_WIDTH as u16,
            height: CANVAS_HEIGHT as u16,
            name: String::from("Canvas"),
        };
        gamestate.canvas_framebuffer = Some(framebuffer_info.clone());
        draw_commands.push(DrawCommand::CreateFramebuffer {
            framebuffer_info: framebuffer_info,
        });
    }
    draw_commands
}

pub fn update_and_draw(input: &GameInput, gamestate: &mut GameState) -> Vec<DrawCommand> {
    // TODO(JaSc): Maybe we additionally want something like SystemCommands that tell the platform
    //             layer to create framebuffers / go fullscreen / turn on vsync / upload textures
    let mut draw_commands = Vec::new();

    if input.hotreload_happened {
        reinitialize_after_hotreload();
    }
    if input.do_reinit_gamestate {
        reinitialize_gamestate(gamestate);
    }
    if input.do_reinit_drawstate {
        let mut initialization_commands = reinitialize_drawstate(gamestate);
        draw_commands.append(&mut initialization_commands);
    }

    let delta = pretty_format_duration_ms(input.time_delta as f64);
    let draw = pretty_format_duration_ms(input.time_draw as f64);
    let update = pretty_format_duration_ms(input.time_update as f64);
    trace!("delta: {}, draw: {}, update: {}", delta, draw, update);

    let texture_atlas = gamestate
        .texture_atlas
        .clone()
        .expect("Texture atlas does not exist");
    let canvas_framebuffer = gamestate
        .canvas_framebuffer
        .clone()
        .expect("Canvas framebuffer does not exist");

    // TODO(JaSc): Change letterbox color based on debug/release
    let letterbox_color = Color::new(0.2, 0.9, 0.4, 1.0);
    draw_commands.push(DrawCommand::Clear {
        framebuffer: FramebufferTarget::Screen,
        color: letterbox_color,
    });

    let canvas_color = Color::new(1.0, 0.4, 0.7, 1.0);
    draw_commands.push(DrawCommand::Clear {
        framebuffer: FramebufferTarget::Offscreen(canvas_framebuffer.clone()),
        color: canvas_color,
    });

    if gamestate.screen_dim != input.screen_dim {
        gamestate.screen_dim = input.screen_dim;
        let screen_rect = Rect::from_dimension(gamestate.screen_dim);
        let canvas_rect = Rect::from_width_height(CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32);
        let blit_rect = canvas_blit_rect(screen_rect, canvas_rect);

        info!("=====================");
        info!("Window resized: {:?}", screen_rect.dim);
        info!("Canvas size: {:?}", canvas_rect.dim);
        info!("Blit-rect: {:?}", blit_rect);
        info!(
            "Pixel scale factor: {} ",
            if blit_rect.pos.x == 0.0 {
                screen_rect.width() / canvas_rect.width()
            } else {
                screen_rect.height() / canvas_rect.height()
            }
        );
        info!(
            "Pixel waste: {} x {}",
            screen_rect.width() - blit_rect.width(),
            screen_rect.height() - blit_rect.height(),
        );
        info!("=====================");
    }

    // ---------------------------------------------------------------------------------------------
    // Mouse input
    //
    let screen_rect = Rect::from_dimension(gamestate.screen_dim);
    let canvas_rect = Rect::from_width_height(CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32);

    // Canvas mouse position
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
    let new_mouse_pos_canvas = screen_coord_to_canvas_coord(
        input.mouse_pos_screen + Vec2::new(0.5, 0.5),
        screen_rect,
        canvas_rect,
    );

    let mouse_delta_canvas = new_mouse_pos_canvas - gamestate.mouse_pos_canvas;
    gamestate.mouse_pos_canvas = new_mouse_pos_canvas;

    // World mouse position
    let new_mouse_pos_world = gamestate.cam.canvas_to_world(new_mouse_pos_canvas);
    let _mouse_delta_world = new_mouse_pos_world - gamestate.mouse_pos_world;
    gamestate.mouse_pos_world = new_mouse_pos_world;

    dprintln!(new_mouse_pos_world);
    dprintln!(new_mouse_pos_world.pixel_snapped());

    if input.mouse_button_right.is_pressed {
        gamestate.cam.pan(mouse_delta_canvas);
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
    let line_start = gamestate.origin;
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
        gamestate.sprite_map["images/plain"].with_scale(Vec2::new(1.0, 1.0) / UNIT_SIZE),
        new_mouse_pos_world.pixel_snapped(),
        -0.1,
        cursor_color,
    );

    // Grid
    let grid_dark = Color::new(0.5, 0.3, 0.0, 1.0);
    let grid_light = Color::new(0.9, 0.7, 0.2, 1.0);
    for x in -30..30 {
        for diagonal in -20..20 {
            let pos =
                Point::new((x + diagonal) as f32, diagonal as f32) * UNIT_SIZE + gamestate.origin;
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

    let draw_target = if input.direct_screen_drawing {
        FramebufferTarget::Screen
    } else {
        FramebufferTarget::Offscreen(canvas_framebuffer.clone())
    };

    draw_commands.push(DrawCommand::from_lines(
        transform,
        texture_atlas.clone(),
        draw_target.clone(),
        line_batch,
    ));
    draw_commands.push(DrawCommand::from_quads(
        transform,
        texture_atlas.clone(),
        draw_target,
        fill_batch,
    ));

    if !input.direct_screen_drawing {
        draw_commands.push(DrawCommand::BlitFramebuffer {
            source_framebuffer: canvas_framebuffer.clone(),
            target_framebuffer: FramebufferTarget::Screen,
            source_rect: canvas_rect,
            target_rect: canvas_blit_rect(screen_rect, canvas_rect),
        });
    }

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

fn pretty_format_duration_ms(duration: f64) -> String {
    format!("{:.3}ms", (duration * 1_000_000.0).round() / 1000.0)
}
