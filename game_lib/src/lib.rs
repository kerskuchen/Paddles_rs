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
pub mod collision;
pub mod draw;
pub mod math;
mod scenes;

pub type ResourcePath = String;

pub use collision::*;
pub use draw::*;
pub use math::*;
use scenes::*;

//==================================================================================================
// SystemCommand

pub enum SystemCommand {
    EnableRelativeMouseMovementCapture(bool),
    ShutdownGame,
}

//==================================================================================================
// GameContext
//==================================================================================================
//
const UNIT_SIZE: f32 = 16.0;
const CANVAS_WIDTH: f32 = 480.0;
const CANVAS_HEIGHT: f32 = 270.0;

const LOG_LEVEL_GENERAL: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_GAME_LIB: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_MATH: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_DRAW: log::LevelFilter = log::LevelFilter::Trace;

#[derive(Default)]
pub struct GameContext<'game_context> {
    globals: Globals,

    gameplay_scene: GameplayScene,
    menu_scene: MenuScene,
    debug_scene: DebugScene,

    drawcontext: DrawContext<'game_context>,
    system_commands: Vec<SystemCommand>,
}

impl<'game_context> GameContext<'game_context> {
    pub fn get_draw_commands(&mut self) -> Vec<DrawCommand> {
        std::mem::replace(&mut self.drawcontext.draw_commands, Vec::new())
    }

    pub fn get_system_commands(&mut self) -> Vec<SystemCommand> {
        std::mem::replace(&mut self.system_commands, Vec::new())
    }

    pub fn new() -> GameContext<'game_context> {
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
    pub game_paused: bool,
    pub fast_time: i32,

    /// Mouse position is given in the following interval:
    /// [0 .. screen_width - 1] x [0 .. screen_height - 1]
    /// where (0,0) is the top left of the screen
    pub mouse_pos_screen: Point,
    pub mouse_delta_screen: Vec2,

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

// TODO(JaSc): Maybe we additionally want something like SystemCommands that tell the platform
//             layer to create framebuffers / go fullscreen / turn on vsync / upload textures
pub fn update_and_draw<'game_context>(
    input: &GameInput,
    gc: &'game_context mut GameContext<'game_context>,
) {
    // ---------------------------------------------------------------------------------------------
    // Init / re-init
    //
    if input.hotreload_happened {
        // Initializing logger
        // NOTE: When hot reloading the game lib dll the logging must be reinitialized
        // TODO(JaSc): Do we actually need the logging?
        //
        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!("{}: {}", record.level(), message))
            })
            .level(LOG_LEVEL_GENERAL)
            .level_for("game_lib", LOG_LEVEL_GAME_LIB)
            .level_for("game_lib::math", LOG_LEVEL_MATH)
            .level_for("game_lib::draw", LOG_LEVEL_DRAW)
            .chain(std::io::stdout())
            .apply()
            .expect("Could not initialize logger");
    }

    if input.do_reinit_gamestate {
        gc.globals.cam = Camera::new(
            WorldPoint::zero(),
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            DEFAULT_WORLD_ZNEAR,
            DEFAULT_WORLD_ZFAR,
        );
        gc.globals.error_happened = None;
        gc.gameplay_scene.reinitialize(&mut gc.system_commands);
        gc.debug_scene.reinitialize(&mut gc.system_commands);
        gc.menu_scene.reinitialize(&mut gc.system_commands);
    }

    if input.do_reinit_drawstate {
        let canvas_dim = if !input.direct_screen_drawing {
            (CANVAS_WIDTH as u16, CANVAS_HEIGHT as u16)
        } else {
            (input.screen_dim.x as u16, input.screen_dim.y as u16)
        };
        gc.drawcontext.reinitialize(canvas_dim.0, canvas_dim.1);
    }

    // ---------------------------------------------------------------------------------------------
    // Mouse input and camera
    //
    let screen_rect = Rect::from_dimension(input.screen_dim);
    let canvas_rect = Rect::from_width_height(CANVAS_WIDTH, CANVAS_HEIGHT);
    let canvas_blit_rect = canvas_blit_rect(screen_rect, canvas_rect);

    // Canvas mouse position
    // TODO(JaSc): new_mouse_pos_canvas and accumulations of new_mouse_delta_canvas will go
    //             out of sync due to rounding errors. Maybe only allow just one or the other
    //             when we get to implement event based input?
    let new_mouse_pos_canvas =
        screen_pos_to_canvas_pos(input.mouse_pos_screen, screen_rect, canvas_rect);
    let new_mouse_delta_canvas =
        screen_vec_to_canvas_vec(input.mouse_delta_screen, screen_rect, canvas_rect);

    // World mouse position
    let new_mouse_pos_world = gc
        .globals
        .cam
        .canvas_point_to_world_point(new_mouse_pos_canvas);
    let new_mouse_delta_world = gc
        .globals
        .cam
        .canvas_vec_to_world_vec(new_mouse_delta_canvas);

    // Camera movement
    if input.mouse_button_right.is_pressed {
        gc.globals.cam.pan(new_mouse_delta_canvas);
    }

    if input.mouse_button_middle.is_pressed {
        gc.globals.cam.zoom_to_world_point(new_mouse_pos_world, 1.0);
    }

    if input.mouse_wheel_delta > 0 {
        let new_zoom_level = f32::min(gc.globals.cam.zoom_level * 2.0, 8.0);
        gc.globals
            .cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    } else if input.mouse_wheel_delta < 0 {
        let new_zoom_level = f32::max(gc.globals.cam.zoom_level / 2.0, 1.0 / 32.0);
        gc.globals
            .cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    }

    gc.globals.mouse_pos_world = new_mouse_pos_world;
    gc.globals.mouse_pos_canvas = new_mouse_pos_canvas;

    gc.globals.mouse_delta_world = new_mouse_delta_world;
    gc.globals.mouse_delta_canvas = new_mouse_delta_canvas;

    // ---------------------------------------------------------------------------------------------
    // Update and draw scenes
    //
    let mut dc = &mut gc.drawcontext;
    dc.start_drawing();
    {
        //_do_collision_tests(dc, new_mouse_pos_world);
        gc.gameplay_scene
            .update_and_draw(input, &mut gc.globals, &mut dc, &mut gc.system_commands);

        gc.menu_scene
            .update_and_draw(input, &mut gc.globals, &mut dc, &mut gc.system_commands);

        gc.debug_scene
            .update_and_draw(input, &mut gc.globals, &mut dc, &mut gc.system_commands);
    }
    let transform = gc.globals.cam.proj_view_matrix();
    dc.finish_drawing(transform, canvas_rect, canvas_blit_rect);
}

// Some intersection tests
fn _do_collision_tests(dc: &mut DrawContext, mouse_pos_world: WorldPoint) {
    let mouse_ray = Line::new(Vec2::zero(), mouse_pos_world);
    let mouse_rect = Rect::from_point_dimension(mouse_pos_world, Vec2::ones() * 2.0);

    // Rect
    let test_rect = Rect {
        left: -8.0 * UNIT_SIZE,
        right: -3.0 * UNIT_SIZE,
        top: 1.0 * UNIT_SIZE,
        bottom: 5.0 * UNIT_SIZE,
    };

    dc.draw_rect_filled(
        Rect::from_point_dimension(Point::new(0.0, 0.0), Vec2::ones()),
        -0.2,
        COLOR_GREEN,
        DrawSpace::World,
    );
    dc.draw_rect_filled(
        Rect::from_point_dimension(Point::new(1.0, 0.0), Vec2::ones()),
        -0.2,
        COLOR_BLUE,
        DrawSpace::World,
    );
    dc.draw_rect_filled(
        Rect::from_point_dimension(Point::new(0.0, 1.0), Vec2::ones()),
        -0.2,
        COLOR_BLUE,
        DrawSpace::World,
    );
    dc.draw_rect_filled(
        Rect::from_point_dimension(Point::new(1.0, 1.0), Vec2::ones()),
        -0.2,
        COLOR_GREEN,
        DrawSpace::World,
    );

    dc.draw_lines(
        &mouse_rect.to_border_lines(),
        0.0,
        if mouse_rect.intersects_rect(test_rect) {
            COLOR_YELLOW
        } else {
            COLOR_BLACK
        },
        DrawSpace::World,
    );

    dc.draw_lines(
        &test_rect.to_border_lines(),
        0.0,
        if mouse_ray.intersects_rect(test_rect) {
            COLOR_RED
        } else {
            COLOR_BLACK
        },
        DrawSpace::World,
    );

    let intersections = intersections_line_rect(mouse_ray, test_rect);
    for intersection in &intersections {
        if let Some(intersection) = intersection {
            dc.draw_rect_filled(
                Rect::from_point_dimension(intersection.point, Vec2::ones()).centered(),
                -0.1,
                COLOR_CYAN,
                DrawSpace::World,
            );
            dc.draw_arrow(
                intersection.point,
                intersection.normal,
                UNIT_SIZE,
                -0.2,
                COLOR_GREEN,
                DrawSpace::World,
            );
        }
    }

    // Line
    let test_line = Line::new(
        Point::new(-4.0, -5.0) * UNIT_SIZE,
        Point::new(-8.0, -2.0) * UNIT_SIZE,
    );
    dc.draw_line(
        test_line,
        0.0,
        if mouse_ray.intersects_line(test_line) {
            COLOR_RED
        } else {
            COLOR_BLACK
        },
        DrawSpace::World,
    );
    if let Some(intersection) = intersection_line_line(mouse_ray, test_line) {
        dc.draw_rect_filled(
            Rect::from_point_dimension(intersection.point, Vec2::ones()).centered(),
            -0.1,
            COLOR_CYAN,
            DrawSpace::World,
        );
        dc.draw_arrow(
            intersection.point,
            intersection.normal,
            UNIT_SIZE,
            -0.2,
            COLOR_GREEN,
            DrawSpace::World,
        );
    }

    // Sphere
    let test_sphere = Circle::new(Vec2::ones() * 3.0 * UNIT_SIZE, UNIT_SIZE);
    dc.draw_lines(
        &test_sphere.to_lines(32),
        0.0,
        if mouse_ray.intersects_circle(test_sphere) {
            COLOR_RED
        } else {
            COLOR_BLACK
        },
        DrawSpace::World,
    );
    dc.draw_arrow(
        Point::zero(),
        mouse_ray.dir().normalized(),
        mouse_ray.length(),
        0.0,
        COLOR_BLUE,
        DrawSpace::World,
    );
    let (near, far) = intersections_line_circle(mouse_ray, test_sphere);
    if let Some(intersection) = near {
        dc.draw_rect_filled(
            Rect::from_point_dimension(intersection.point, Vec2::ones()).centered(),
            -0.1,
            COLOR_MAGENTA,
            DrawSpace::World,
        );
        dc.draw_arrow(
            intersection.point,
            intersection.normal,
            UNIT_SIZE,
            -0.2,
            COLOR_GREEN,
            DrawSpace::World,
        );
    }
    if let Some(intersection) = far {
        dc.draw_rect_filled(
            Rect::from_point_dimension(intersection.point, Vec2::ones()).centered(),
            -0.1,
            COLOR_CYAN,
            DrawSpace::World,
        );
        dc.draw_arrow(
            intersection.point,
            intersection.normal,
            UNIT_SIZE,
            -0.2,
            COLOR_GREEN,
            DrawSpace::World,
        );
    }
}

// =================================================================================================
// TODO(JaSc): Find a better place for the following functions
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

    (canvas_rect.dim() * ((clamped_point - blit_rect.pos()) / blit_rect.dim())).pixel_snapped()
}

fn screen_vec_to_canvas_vec(screen_vec: Vec2, screen_rect: Rect, canvas_rect: Rect) -> CanvasVec {
    (canvas_rect.dim() * (screen_vec / screen_rect.dim())).pixel_snapped()
}

fn pretty_format_duration_ms(duration: f64) -> String {
    format!("{:.3}ms", (duration * 1_000_000.0).round() / 1000.0)
}
