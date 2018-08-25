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

pub type ResourcePath = String;

// TODO(JaSc): We need more consistency in names: Is it FrameBuffer or Framebuffer?
//             Is it GameState or Gamestate? When its GameState why do variables are then called
//             gamestate and not game_state?
pub use draw::{
    vertices_from_rects, Animation, AtlasMeta, ComponentBytes, DrawCommand, DrawContext, Font,
    FramebufferInfo, FramebufferTarget, Glyph, LineMesh, Mesh, Pixel, PolygonMesh, Sprite,
    TextureInfo, Vertex, VertexIndex,
};
pub use math::{
    Camera, CanvasPoint, Color, Line, Mat4, Mat4Helper, Point, Rect, SquareMatrix, Vec2, WorldPoint,
};

const UNIT_SIZE: f32 = 16.0;
const CANVAS_WIDTH: f32 = 480.0;
const CANVAS_HEIGHT: f32 = 270.0;

const LOG_LEVEL_GENERAL: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_GAME_LIB: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_MATH: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_DRAW: log::LevelFilter = log::LevelFilter::Trace;

#[derive(Default)]
pub struct GameState<'gamestate> {
    screen_dim: Vec2,

    drawcontext: DrawContext<'gamestate>,

    mouse_pos_canvas: CanvasPoint,
    mouse_pos_world: WorldPoint,

    origin: WorldPoint,
    cam: Camera,
}

impl<'gamestate> GameState<'gamestate> {
    pub fn get_draw_commands(&mut self) -> Vec<DrawCommand> {
        std::mem::replace(&mut self.drawcontext.draw_commands, Vec::new())
    }

    pub fn new() -> GameState<'gamestate> {
        Default::default()
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
    /// where (0,0) is the top left of the screen
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
        Default::default()
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
        Default::default()
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
    gamestate.cam = Camera::new(gamestate.origin, CANVAS_WIDTH, CANVAS_HEIGHT, -1.0, 1.0);
}

// TODO(JaSc): Maybe we additionally want something like SystemCommands that tell the platform
//             layer to create framebuffers / go fullscreen / turn on vsync / upload textures
pub fn update_and_draw<'gamestate>(
    input: &GameInput,
    gamestate: &'gamestate mut GameState<'gamestate>,
) {
    if input.hotreload_happened {
        reinitialize_after_hotreload();
    }
    if input.do_reinit_gamestate {
        reinitialize_gamestate(gamestate);
    }

    if input.do_reinit_drawstate {
        let canvas_dim = if !input.direct_screen_drawing {
            (CANVAS_WIDTH as u16, CANVAS_HEIGHT as u16)
        } else {
            (input.screen_dim.x as u16, input.screen_dim.y as u16)
        };
        gamestate
            .drawcontext
            .reinitialize(canvas_dim.0, canvas_dim.1);
    }

    let delta = pretty_format_duration_ms(f64::from(input.time_delta));
    let draw = pretty_format_duration_ms(f64::from(input.time_draw));
    let update = pretty_format_duration_ms(f64::from(input.time_update));
    trace!("delta: {}, draw: {}, update: {}", delta, draw, update);

    // ---------------------------------------------------------------------------------------------
    // Screen size changed
    //
    if gamestate.screen_dim != input.screen_dim {
        gamestate.screen_dim = input.screen_dim;
        let screen_rect = Rect::from_dimension(gamestate.screen_dim);
        let canvas_rect = Rect::from_width_height(CANVAS_WIDTH, CANVAS_HEIGHT);
        let blit_rect = canvas_blit_rect(screen_rect, canvas_rect);

        info!("=====================");
        info!("Window resized: {:?}", screen_rect.dim());
        info!("Canvas size: {:?}", canvas_rect.dim());
        info!("Blit-rect: {:?}", blit_rect);
        info!(
            "Pixel scale factor: {} ",
            if blit_rect.left == 0.0 {
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
    let canvas_rect = Rect::from_width_height(CANVAS_WIDTH, CANVAS_HEIGHT);
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
            Rect::from_point_dimension(new_mouse_pos_world.pixel_snapped(), Vec2::ones()),
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
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE),
                        -0.9,
                        grid_light,
                    );
                } else {
                    let r = (x + 30) as f32 / 60.0;
                    let g = (diagonal + 20) as f32 / 40.0;
                    let b = (r + g) / 2.0;
                    drawcontext.draw_rect_filled(
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE),
                        -0.9,
                        Color::new(r, g, b, 1.0),
                    );
                }
            }
        }

        for y in 0..10 {
            for x in 0..10 {
                let r = (x) as f32 / 10.0;
                let g = (y) as f32 / 10.0;
                let b = 0.0; //(r + g) / 2.0;
                drawcontext.draw_rect_filled(
                    Rect::from_point_dimension(
                        Point::new(x as f32, y as f32) * UNIT_SIZE,
                        Vec2::ones() * UNIT_SIZE / 2.0,
                    ),
                    -0.9,
                    Color::new(r, g, b, 1.0),
                );
            }
        }

        let rect =
            Rect::from_xy_width_height(-5.0 * UNIT_SIZE, 0.0, 4.0 * UNIT_SIZE, 4.0 * UNIT_SIZE);

        drawcontext.debug_draw_rect_textured(rect, -0.05, Color::new(1.0, 1.0, 1.0, 1.0));
        let line = Line {
            start: Point::new(rect.left, rect.bottom),
            end: Point::new(rect.right, rect.top),
        };
        drawcontext.draw_line(line, -0.04, Color::new(1.0, 0.0, 0.0, 1.0));

        drawcontext.debug_draw_rect_textured(
            Rect::from_point_dimension(Point::zero(), Vec2::ones() * UNIT_SIZE),
            -0.1,
            Color::new(0.0, 0.0, 0.0, 0.0),
        );

        // Playing field
        let field_bounds = Rect {
            left: -11.0 * UNIT_SIZE,
            right: 11.0 * UNIT_SIZE,
            top: -7.0 * UNIT_SIZE,
            bottom: 7.0 * UNIT_SIZE,
        };
        let field_depth = -0.8;
        let field_border_lines = field_bounds.to_border_lines();

        for (&line, &color) in field_border_lines.iter().zip(
            [
                draw::COLOR_YELLOW,
                draw::COLOR_MAGENTA,
                draw::COLOR_WHITE,
                draw::COLOR_CYAN,
            ].iter(),
        ) {
            drawcontext.draw_line(line, field_depth, color);
        }

        let field_border_left = Rect {
            left: field_bounds.left - UNIT_SIZE,
            right: field_bounds.left,
            top: field_bounds.top,
            bottom: field_bounds.bottom,
        };
        let field_border_right = Rect {
            left: field_bounds.right,
            right: field_bounds.right + UNIT_SIZE,
            bottom: field_bounds.bottom,
            top: field_bounds.top,
        };
        let field_border_top = Rect {
            left: field_bounds.left - UNIT_SIZE,
            right: field_bounds.right + UNIT_SIZE,
            top: field_bounds.top - UNIT_SIZE,
            bottom: field_bounds.top,
        };
        let field_border_bottom = Rect {
            left: field_bounds.left - UNIT_SIZE,
            right: field_bounds.right + UNIT_SIZE,
            top: field_bounds.bottom,
            bottom: field_bounds.bottom + UNIT_SIZE,
        };
        for (&field_border, &color) in [
            field_border_left,
            field_border_right,
            field_border_top,
            field_border_bottom,
        ].iter()
            .zip(
                [
                    draw::COLOR_RED,
                    draw::COLOR_GREEN,
                    draw::COLOR_BLUE,
                    draw::COLOR_BLACK,
                ].iter(),
            ) {
            drawcontext.draw_rect_filled(field_border, field_depth, color);
        }
    }
    let transform = gamestate.cam.proj_view_matrix();
    drawcontext.finish_drawing(transform, canvas_rect, canvas_blit_rect);
}

// =================================================================================================
// TODO(JaSc): Find a better place for the following three functions
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
        .centered_in_rect(screen_rect)
}

// TODO(JaSc): Proofread and refactor this
/// Clamps a given `screen_point` to the area of the [`canvas_blit_rect`] and converts the result
/// into a canvas-position in the following interval:
/// `[0..canvas_rect.width-1]x[0..canvas_rect.height-1]`
/// where `(0,0)` is the top left of the canvas.
fn screen_pos_to_canvas_pos(screen_point: Point, screen_rect: Rect, canvas_rect: Rect) -> Point {
    // NOTE: Clamping the point needs to use integer arithmetic such that
    //          x != canvas.rect.width and y != canvas.rect.height
    //       holds. We therefore need to subtract one from the blit_rect's dimension and then
    //       add one again after clamping to achieve the desired effect.
    // TODO(JaSc): Maybe make this more self documenting via integer rectangles
    let mut blit_rect = canvas_blit_rect(screen_rect, canvas_rect);
    blit_rect.right -= 1.0;
    blit_rect.bottom -= 1.0;
    let clamped_point = screen_point.clamped_in_rect(blit_rect);
    blit_rect.right += 1.0;
    blit_rect.bottom += 1.0;

    let result = canvas_rect.dim() * ((clamped_point - blit_rect.pos()) / blit_rect.dim());
    Point::new(f32::floor(result.x), f32::floor(result.y))
}

fn pretty_format_duration_ms(duration: f64) -> String {
    format!("{:.3}ms", (duration * 1_000_000.0).round() / 1000.0)
}
