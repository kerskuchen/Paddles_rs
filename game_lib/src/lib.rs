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

#[macro_use]
pub mod utility;
pub mod draw;
pub mod math;

// TODO(JaSc): We need more consistency in names: Is it FrameBuffer or Framebuffer?
//             Is it GameState or Gamestate? When its GameState why do variables are then called
//             gamestate and not game_state?
pub use draw::{
    ComponentBytes, DrawCommand, DrawContext, DrawMode, FramebufferInfo, FramebufferTarget,
    LineBatch, Pixel, Quad, QuadBatch, Sprite, TextureInfo, Vertex, VertexIndex,
};
pub use math::{
    Bounds, Camera, CanvasPoint, Color, Line, Mat4, Mat4Helper, Point, Rect, SquareMatrix, Vec2,
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

    drawcontext: DrawContext,

    mouse_pos_canvas: CanvasPoint,
    mouse_pos_world: WorldPoint,

    origin: WorldPoint,
    cam: Camera,
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            screen_dim: Vec2::zero(),

            drawcontext: DrawContext::new(),

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
    gamestate.origin = WorldPoint::zero();
    gamestate.cam = Camera::with_position(gamestate.origin, CANVAS_WIDTH, CANVAS_HEIGHT, -1.0, 1.0);
}

// TODO(JaSc): Maybe we additionally want something like SystemCommands that tell the platform
//             layer to create framebuffers / go fullscreen / turn on vsync / upload textures
pub fn update_and_draw(input: &GameInput, gamestate: &mut GameState) -> Vec<DrawCommand> {
    if input.hotreload_happened {
        reinitialize_after_hotreload();
    }
    if input.do_reinit_gamestate {
        reinitialize_gamestate(gamestate);
    }

    if input.do_reinit_drawstate {
        if !input.direct_screen_drawing {
            gamestate
                .drawcontext
                .reinitialize(CANVAS_WIDTH as u16, CANVAS_HEIGHT as u16);
        } else {
            gamestate
                .drawcontext
                .reinitialize(input.screen_dim.x as u16, input.screen_dim.y as u16);
        }
    }

    let delta = pretty_format_duration_ms(f64::from(input.time_delta));
    let draw = pretty_format_duration_ms(f64::from(input.time_draw));
    let update = pretty_format_duration_ms(f64::from(input.time_update));
    trace!("delta: {}, draw: {}, update: {}", delta, draw, update);

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
    let canvas_blit_rect = canvas_blit_rect(screen_rect, canvas_rect);

    // Canvas mouse position
    let new_mouse_pos_canvas =
        screen_pos_to_canvas_pos(input.mouse_pos_screen, screen_rect, canvas_rect);
    let mouse_delta_canvas = new_mouse_pos_canvas - gamestate.mouse_pos_canvas;
    gamestate.mouse_pos_canvas = new_mouse_pos_canvas;

    // World mouse position
    let new_mouse_pos_world = gamestate.cam.canvas_to_world(new_mouse_pos_canvas);
    let _mouse_delta_world = new_mouse_pos_world - gamestate.mouse_pos_world;
    gamestate.mouse_pos_world = new_mouse_pos_world;

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

    let drawcontext = &mut gamestate.drawcontext;
    drawcontext.start_drawing();
    {
        // Draw line from origin to cursor position
        drawcontext.draw_line(
            Line::new(
                gamestate.origin,
                new_mouse_pos_world.pixel_snapped() + Vec2::ones() * 0.5,
            ),
            0.0,
            Color::new(1.0, 0.0, 0.0, 1.0),
        );

        // Draw cursor
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
        drawcontext.draw_rect_filled(
            Rect::from_point_dimension(new_mouse_pos_world.pixel_snapped(), Vec2::ones())
                .to_bounds(),
            -0.1,
            cursor_color,
        );

        // Draw grid
        let grid_dark = Color::new(0.5, 0.3, 0.0, 1.0);
        let grid_light = Color::new(0.9, 0.7, 0.2, 1.0);
        for x in -30..30 {
            for diagonal in -20..20 {
                let pos = Point::new((x + diagonal) as f32, diagonal as f32) * UNIT_SIZE
                    + gamestate.origin;
                if x % 2 == 0 {
                    drawcontext.draw_rect_filled(
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE).to_bounds(),
                        -0.9,
                        grid_light,
                    );
                } else {
                    drawcontext.draw_rect_filled(
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE).to_bounds(),
                        -0.9,
                        grid_dark,
                    );
                }
            }
        }

        // Playing field
        let field_bounds = Bounds {
            left: -11.0 * UNIT_SIZE,
            right: 11.0 * UNIT_SIZE,
            bottom: -7.0 * UNIT_SIZE,
            top: 7.0 * UNIT_SIZE,
        };
        let field_depth = -0.8;
        let field_border_color = Color::new(0.7, 0.3, 0.3, 1.0);
        let field_border_line_color = Color::new(1.0, 0.0, 0.0, 1.0);
        let field_border_lines = field_bounds.to_border_lines();

        for &line in &field_border_lines {
            drawcontext.draw_line(line, field_depth, field_border_line_color);
        }
        let field_border_left = Bounds {
            left: field_bounds.left - UNIT_SIZE,
            right: field_bounds.left,
            bottom: field_bounds.bottom,
            top: field_bounds.top,
        };
        let field_border_right = Bounds {
            left: field_bounds.right,
            right: field_bounds.right + UNIT_SIZE,
            bottom: field_bounds.bottom,
            top: field_bounds.top,
        };
        let field_border_top = Bounds {
            left: field_bounds.left - UNIT_SIZE,
            right: field_bounds.right + UNIT_SIZE,
            bottom: field_bounds.top,
            top: field_bounds.top + UNIT_SIZE,
        };
        let field_border_bottom = Bounds {
            left: field_bounds.left - UNIT_SIZE,
            right: field_bounds.right + UNIT_SIZE,
            bottom: field_bounds.bottom - UNIT_SIZE,
            top: field_bounds.bottom,
        };
        for &field_border in &[
            field_border_left,
            field_border_right,
            field_border_top,
            field_border_bottom,
        ] {
            drawcontext.draw_rect_filled(field_border, field_depth, field_border_color);
        }
    }
    let transform = gamestate.cam.proj_view_matrix();
    drawcontext.finish_drawing(transform, canvas_rect, canvas_blit_rect)
}

// =================================================================================================
// TODO(JaSc): Find a better place for the following two functions
// =================================================================================================

/// Returns the `blit_rectangle` of for given canvas and screen rectangles.
/// The `blit-rectange` is the area of the screen where the content of the canvas is drawn onto.
/// It is as big as the canvas proportionally stretched and centered to fill the whole
/// screen.
///
/// It may or may not be smaller than the full screen size depending on the aspect
/// ratio of both the screen and the canvas. The `blit_rectange` is guaranteed to either have
/// the same width a as the screen (with letterboxing if needed) or the same height as the
/// screen (with columnboxing if needed) or completely fill the screen.
///
/// # Examples
/// ```
/// // +------+  +--------------+  +---------------+
/// // |canvas|  |   screen     |  |               | <- screen
/// // | 8x4  |  |    16x12     |  +---------------+
/// // +------+  |              |  |   blit-rect   |
/// //           |              |  |     16x10     |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  +---------------+
/// //           |              |  |               |
/// //           +--------------+  +---------------+
/// //
/// // +------+  +----------------+  +-+-------------+-+
/// // |canvas|  |     screen     |  | |             | |
/// // | 8x4  |  |      18x8      |  | |             | |
/// // +------+  |                |  | |  blit-rect  | |
/// //           |                |  | |    16x8     | |
/// //           |                |  | |             | |
/// //           |                |  | |             | |
/// //           +----------------+  +-+-------------+-+
/// //                                                ^---- screen
/// //
/// // +------+  +----------------+  +-----------------+
/// // |canvas|  |     screen     |  |                 |
/// // | 8x4  |  |      16x8      |  |                 |
/// // +------+  |                |  |    blit-rect    |
/// //           |                |  |      16x8       |
/// //           |                |  |                 |
/// //           |                |  |                 |
/// //           +----------------+  +-----------------+
/// //                                                ^---- blit-rect == screen
/// ```
pub fn canvas_blit_rect(screen_rect: Rect, canvas_rect: Rect) -> Rect {
    canvas_rect
        .stretched_to_fit(screen_rect)
        .centered_in(screen_rect)
}

// TODO(JaSc): Proofread and refactor this
/// Clamps a given `screen_point` to the area of the [`canvas_blit_rect`] and converts the result
/// into a canvas-position in the following interval:
/// `[0..canvas_rect.width-1]x[0..canvas_rect.height-1]`
/// where `(0,0)` is the bottom left of the canvas.
fn screen_pos_to_canvas_pos(screen_point: Point, screen_rect: Rect, canvas_rect: Rect) -> Point {
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
