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

pub type ResourcePath = String;

pub use collision::*;
pub use draw::*;
pub use math::*;

//==================================================================================================
// SystemCommand

pub enum SystemCommand {
    EnableRelativeMouseMovementCapture(bool),
}

//==================================================================================================
// GameContext
//==================================================================================================
//
const PONGI_RADIUS: f32 = 7.5;
const PONGI_BASE_SPEED: f32 = 15.0 * UNIT_SIZE;

const WALL_THICKNESS: f32 = 0.5 * UNIT_SIZE;
const PADDLE_SIZE: f32 = 3.0 * UNIT_SIZE;

const FIELD_BOUNDS: Rect = Rect {
    left: -10.0 * UNIT_SIZE,
    right: 10.0 * UNIT_SIZE,
    top: -6.0 * UNIT_SIZE,
    bottom: 6.0 * UNIT_SIZE,
};

const UNIT_SIZE: f32 = 16.0;
const CANVAS_WIDTH: f32 = 480.0;
const CANVAS_HEIGHT: f32 = 270.0;

const LOG_LEVEL_GENERAL: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_GAME_LIB: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_MATH: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_DRAW: log::LevelFilter = log::LevelFilter::Trace;

enum GameState {
    Startup,
    Menu,
    Game,
    Paused,
}

#[derive(Default)]
pub struct GameContext<'game_context> {
    error_happened: Option<String>,

    screen_dim: Vec2,

    drawcontext: DrawContext<'game_context>,
    system_commands: Vec<SystemCommand>,

    time_till_next_beat: f32,

    paddle_left_pos: f32,
    paddle_right_pos: f32,

    pongi_pos: WorldPoint,
    pongi_vel: Vec2,

    mouse_pos_canvas: CanvasPoint,
    mouse_pos_world: WorldPoint,

    is_in_menu: bool,

    origin: WorldPoint,
    cam: Camera,
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
    pub mouse_delta_pos_screen: Vec2,

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
    // TODO(JaSc): Do we actually need the logging?
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

fn reinitialize_gamestate(gc: &mut GameContext) {
    gc.origin = WorldPoint::zero();
    gc.cam = Camera::new(
        gc.origin,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        DEFAULT_WORLD_ZNEAR,
        DEFAULT_WORLD_ZFAR,
    );

    //gc.pongi_pos = Point::new(0.0, -3.0 * UNIT_SIZE);
    //gc.pongi_vel = Vec2::new(0.0, -5.0 * UNIT_SIZE);

    let angle: f32 = 40.0;
    gc.pongi_pos = Point::new(8.0, -4.0) * UNIT_SIZE;
    gc.pongi_vel = Vec2::from_angle(angle.to_radians()) * PONGI_BASE_SPEED;
    gc.error_happened = None;

    // gc.pongi_pos = Point::new(-151.48575, -88.0);
    // gc.pongi_vel = Vec2::new(-4644.807, 6393.034);

    gc.system_commands
        .push(SystemCommand::EnableRelativeMouseMovementCapture(true));
}

// TODO(JaSc): Maybe we additionally want something like SystemCommands that tell the platform
//             layer to create framebuffers / go fullscreen / turn on vsync / upload textures
pub fn update_and_draw<'game_context>(
    input: &GameInput,
    gc: &'game_context mut GameContext<'game_context>,
) {
    if input.hotreload_happened {
        reinitialize_after_hotreload();
    }
    if input.do_reinit_gamestate {
        reinitialize_gamestate(gc);
    }

    if input.do_reinit_drawstate {
        let canvas_dim = if !input.direct_screen_drawing {
            (CANVAS_WIDTH as u16, CANVAS_HEIGHT as u16)
        } else {
            (input.screen_dim.x as u16, input.screen_dim.y as u16)
        };
        gc.drawcontext.reinitialize(canvas_dim.0, canvas_dim.1);
    }

    let delta_time = if input.game_paused || gc.error_happened.is_some() {
        0.0
    } else {
        let time_factor = if input.fast_time == 0 {
            1.0
        } else if input.fast_time > 0 {
            (input.fast_time + 1) as f32
        } else {
            1.0 / (((i32::abs(input.fast_time)) + 1) as f32)
        };
        input.time_delta * time_factor
    };

    // ---------------------------------------------------------------------------------------------
    // Screen size changed
    //
    if gc.screen_dim != input.screen_dim {
        gc.screen_dim = input.screen_dim;
        let screen_rect = Rect::from_dimension(gc.screen_dim);
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
    let screen_rect = Rect::from_dimension(gc.screen_dim);
    let canvas_rect = Rect::from_width_height(CANVAS_WIDTH, CANVAS_HEIGHT);
    let canvas_blit_rect = canvas_blit_rect(screen_rect, canvas_rect);

    // Canvas mouse position
    let new_mouse_pos_canvas =
        screen_pos_to_canvas_pos(input.mouse_pos_screen, screen_rect, canvas_rect);
    let mouse_delta_canvas = new_mouse_pos_canvas - gc.mouse_pos_canvas;
    gc.mouse_pos_canvas = new_mouse_pos_canvas;

    // World mouse position
    let new_mouse_pos_world = gc.cam.canvas_to_world(new_mouse_pos_canvas);
    let _mouse_delta_world = new_mouse_pos_world - gc.mouse_pos_world;
    gc.mouse_pos_world = new_mouse_pos_world;

    if input.mouse_button_right.is_pressed {
        gc.cam.pan(mouse_delta_canvas);
    }

    if input.mouse_button_middle.is_pressed {
        gc.cam.zoom_to_world_point(new_mouse_pos_world, 1.0);
    }

    if input.mouse_wheel_delta > 0 {
        let new_zoom_level = f32::min(gc.cam.zoom_level * 2.0, 8.0);
        gc.cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    } else if input.mouse_wheel_delta < 0 {
        let new_zoom_level = f32::max(gc.cam.zoom_level / 2.0, 1.0 / 32.0);
        gc.cam
            .zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
    }
    // ---------------------------------------------------------------------------------------------
    // Generate draw commands
    //
    let dc = &mut gc.drawcontext;
    dc.start_drawing();
    {
        //do_collision_tests(dc, new_mouse_pos_world);

        // ---------------------------------------------------------------------------------------------
        // Playfield
        //

        // Draw grid
        let grid_light = Color::new(0.9, 0.7, 0.2, 1.0);
        for x in -30..30 {
            for diagonal in -20..20 {
                let pos =
                    Point::new((x + diagonal) as f32, diagonal as f32) * UNIT_SIZE + gc.origin;
                if x % 2 == 0 {
                    dc.draw_rect_filled(
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE),
                        -1.0,
                        grid_light,
                        DrawSpace::World,
                    );
                } else {
                    let r = (x + 30) as f32 / 60.0;
                    let g = (diagonal + 20) as f32 / 40.0;
                    let b = (r + g) / 2.0;
                    dc.draw_rect_filled(
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE),
                        -1.0,
                        Color::new(r, g, b, 1.0),
                        DrawSpace::World,
                    );
                }
            }
        }

        // Draw playing field
        let field_depth = -0.4;

        let field_border_left = Rect {
            left: FIELD_BOUNDS.left - WALL_THICKNESS,
            right: FIELD_BOUNDS.left,
            top: FIELD_BOUNDS.top,
            bottom: FIELD_BOUNDS.bottom,
        };
        let field_border_right = Rect {
            left: FIELD_BOUNDS.right,
            right: FIELD_BOUNDS.right + WALL_THICKNESS,
            top: FIELD_BOUNDS.top,
            bottom: FIELD_BOUNDS.bottom,
        };
        let field_border_top = Rect {
            left: FIELD_BOUNDS.left - WALL_THICKNESS,
            right: FIELD_BOUNDS.right + WALL_THICKNESS,
            top: FIELD_BOUNDS.top - WALL_THICKNESS,
            bottom: FIELD_BOUNDS.top,
        };
        let field_border_bottom = Rect {
            left: FIELD_BOUNDS.left - WALL_THICKNESS,
            right: FIELD_BOUNDS.right + WALL_THICKNESS,
            top: FIELD_BOUNDS.bottom,
            bottom: FIELD_BOUNDS.bottom + WALL_THICKNESS,
        };
        // let field_border_center =
        //     Rect::unit_rect_centered().scaled_from_center(Vec2::ones() * UNIT_SIZE);

        for (&field_border, &color) in [
            field_border_left,
            field_border_right,
            field_border_top,
            field_border_bottom,
            //field_border_center,
        ].iter()
            .zip(
                [
                    draw::COLOR_RED,
                    draw::COLOR_GREEN,
                    draw::COLOR_BLUE,
                    draw::COLOR_BLACK,
                    //draw::COLOR_YELLOW,
                ].iter(),
            ) {
            dc.draw_rect_filled(field_border, field_depth, color, DrawSpace::World);
        }

        // Update beat
        const BPM: f32 = 100.0;
        const BEAT_LENGTH: f32 = 60.0 / BPM;

        let mut time_till_next_beat = gc.time_till_next_beat;
        time_till_next_beat -= delta_time;
        while time_till_next_beat < 0.0 {
            time_till_next_beat += BEAT_LENGTH;
        }
        let beat_value = beat_visualizer_value(time_till_next_beat, BEAT_LENGTH);

        // Update pongi
        let pongi_pos = gc.pongi_pos;
        let pongi_vel = gc.pongi_vel;

        let mut collision_mesh = CollisionMesh::new("play_field");
        collision_mesh.add_rect("left_wall", field_border_left);
        collision_mesh.add_rect("right_wall", field_border_right);
        collision_mesh.add_rect("top_wall", field_border_top);
        collision_mesh.add_rect("bottom_wall", field_border_bottom);
        //collision_mesh.add_rect("center_wall", field_border_center);

        let mut error_happened = None;
        let (new_pongi_pos, new_pongi_vel) = move_sphere_with_full_elastic_collision(
            &mut collision_mesh,
            pongi_pos,
            pongi_vel,
            PONGI_RADIUS,
            delta_time,
        ).unwrap_or_else(|error| {
            error_happened = Some(error);
            (pongi_pos, pongi_vel)
        });

        // Write back to game_context
        gc.error_happened = error_happened;
        if gc.error_happened.is_none() {
            gc.pongi_vel = new_pongi_vel;
            gc.pongi_pos = new_pongi_pos;
            gc.time_till_next_beat = time_till_next_beat;
            gc.paddle_left_pos = clamp(
                gc.paddle_left_pos
                    + input.mouse_delta_pos_screen.y / screen_rect.height() * canvas_rect.height(),
                FIELD_BOUNDS.top,
                FIELD_BOUNDS.bottom - PADDLE_SIZE,
            );
        }

        // Debug draw sphere sweeping
        collision_mesh
            .shapes
            .iter()
            .map(|rect| RectSphereSum::new(rect, PONGI_RADIUS))
            .for_each(|sum| dc.draw_lines(&sum.to_lines(), 0.0, COLOR_YELLOW, DrawSpace::World));

        //if let Some(collision) = collision_mesh.sweepcast_sphere(look_ahead_raycast, PONGI_RADIUS) {
        //    println!(
        //        "Intersection with '{}' on shape '{:?}' on segment '{:?}':\n {:?}",
        //        collision_mesh.name, collision.shape, collision.segment, collision.intersection
        //    );
        //}

        // Draw beat visualizer
        let beat_box_pos = Vec2::new(canvas_rect.right - 2.0 * UNIT_SIZE, 1.5 * UNIT_SIZE - 1.0);
        let beat_box_size = UNIT_SIZE * (0.5 + beat_value);
        dc.draw_rect_filled(
            Rect::from_point_dimension(beat_box_pos, Vec2::ones() * beat_box_size).centered(),
            0.0,
            draw::COLOR_MAGENTA,
            DrawSpace::Canvas,
        );

        // Draw pongi
        dc.debug_draw_text(&dformat!(gc.pongi_vel), draw::COLOR_WHITE);
        dc.debug_draw_text(&dformat!(gc.pongi_pos), draw::COLOR_WHITE);
        dc.draw_arrow(
            gc.pongi_pos.pixel_snapped(),
            gc.pongi_vel.normalized(),
            0.3 * gc.pongi_vel.magnitude(),
            -0.1,
            draw::COLOR_GREEN,
            DrawSpace::World,
        );

        dc.debug_draw_circle_textured(
            gc.pongi_pos.pixel_snapped(),
            -0.3,
            Color::new(1.0 - beat_value, 1.0 - beat_value, 1.0, 1.0),
            DrawSpace::World,
        );

        // Draw paddles
        dc.draw_rect_filled(
            Rect::from_point(
                WorldPoint::new(FIELD_BOUNDS.left - WALL_THICKNESS, gc.paddle_left_pos),
                WALL_THICKNESS,
                PADDLE_SIZE,
            ),
            -0.2,
            COLOR_WHITE,
            DrawSpace::World,
        );
        dc.draw_rect_filled(
            Rect::from_point(
                WorldPoint::new(FIELD_BOUNDS.right, gc.paddle_right_pos),
                WALL_THICKNESS,
                PADDLE_SIZE,
            ),
            -0.2,
            COLOR_WHITE,
            DrawSpace::World,
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
        dc.debug_draw_cursor(
            gc.cam.world_to_canvas(new_mouse_pos_world.pixel_snapped()),
            -0.3,
            COLOR_WHITE,
            DrawSpace::Canvas,
        );
        dc.draw_rect_filled(
            Rect::from_point_dimension(new_mouse_pos_world.pixel_snapped(), Vec2::ones()),
            -0.2,
            cursor_color,
            DrawSpace::World,
        );

        // Frametimes etc.
        let delta = pretty_format_duration_ms(f64::from(input.time_delta));
        let draw = pretty_format_duration_ms(f64::from(input.time_draw));
        let update = pretty_format_duration_ms(f64::from(input.time_update));
        dc.debug_draw_text(
            &format!("delta: {}\ndraw: {}\nupdate: {}\n", delta, draw, update),
            draw::COLOR_WHITE,
        );
        dc.debug_draw_text(
            &format!(
                "mouse_screen: {}x{}",
                input.mouse_pos_screen.x, input.mouse_pos_screen.y
            ),
            draw::COLOR_WHITE,
        );
        dc.debug_draw_text(
            &format!(
                "mouse_delta_screen: {}x{}",
                input.mouse_delta_pos_screen.x, input.mouse_delta_pos_screen.y
            ),
            draw::COLOR_WHITE,
        );
        dc.debug_draw_text(
            &format!(
                "mouse_world: {}x{}",
                new_mouse_pos_world.pixel_snapped().x,
                new_mouse_pos_world.pixel_snapped().y
            ),
            draw::COLOR_WHITE,
        );
        dc.debug_draw_text(
            &format!(
                "mouse_canvas: {}x{}\n",
                new_mouse_pos_canvas.pixel_snapped().x,
                new_mouse_pos_canvas.pixel_snapped().y
            ),
            draw::COLOR_WHITE,
        );

        if input.fast_time != 0 {
            if input.fast_time > 0 {
                dc.debug_draw_text(
                    &format!("Time speedup {}x", input.fast_time + 1),
                    draw::COLOR_GREEN,
                );
            } else if input.fast_time < 0 {
                dc.debug_draw_text(
                    &format!("Time slowdown {}x", i32::abs(input.fast_time) + 1),
                    draw::COLOR_YELLOW,
                );
            } else {
            };
        }
        if input.game_paused {
            dc.debug_draw_text("The game is paused", draw::COLOR_CYAN);
        }
        // Debug crash message
        if gc.error_happened.is_some() {
            dc.debug_draw_text(
                &format!(
                    "The game has crashed: {}",
                    gc.error_happened.clone().unwrap()
                ),
                draw::COLOR_RED,
            );
        }
    }
    let transform = gc.cam.proj_view_matrix();
    dc.finish_drawing(transform, canvas_rect, canvas_blit_rect);
}

fn move_sphere_with_full_elastic_collision(
    collision_mesh: &CollisionMesh,
    mut pos: WorldPoint,
    mut vel: Vec2,
    sphere_radius: f32,
    delta_time: f32,
) -> Result<(WorldPoint, Vec2), String> {
    let speed = vel.magnitude();
    let mut dir = vel.normalized();
    let mut travel_distance = speed * delta_time;

    let mut travel_raycast =
        Line::new(pos, pos + (travel_distance + COLLISION_SAFETY_MARGIN) * dir);

    let mut debug_num_loops = 0;
    while let Some(collision) = collision_mesh.sweepcast_sphere(travel_raycast, sphere_radius) {
        // Determine a point that is right before the actual collision point
        let distance_till_hit = (collision.intersection.point - pos).magnitude();
        let safe_collision_point_distance = distance_till_hit - COLLISION_SAFETY_MARGIN;
        if travel_distance < safe_collision_point_distance {
            // We won't hit anything yet in this frame
            break;
        }

        // Move ourselves to the position right before the actual collision point
        pos += safe_collision_point_distance * dir;
        dir = vel
            .normalized()
            .reflected_on_normal(collision.intersection.normal);
        vel = dir * speed;
        travel_distance -= safe_collision_point_distance;

        travel_raycast = Line::new(pos, pos + (travel_distance + COLLISION_SAFETY_MARGIN) * dir);

        debug_num_loops += 1;
        if debug_num_loops == 3 {
            return Err(format!(
                "Collision loop took {} iterations",
                debug_num_loops
            ));
        }
    }
    pos += travel_distance * dir;

    Ok((pos, vel))
}

fn beat_visualizer_value(time_till_next_beat: f32, beat_length: f32) -> f32 {
    let ratio = time_till_next_beat / beat_length;
    let increasing = (1.0 - ratio).powi(10);
    let decreasing = ratio.powi(3);

    increasing + decreasing
}

fn _do_collision_tests(dc: &mut DrawContext, new_mouse_pos_world: WorldPoint) {
    let mouse_ray = Line::new(Vec2::zero(), new_mouse_pos_world);

    // Rect
    let test_rect = Rect {
        left: -8.0 * UNIT_SIZE,
        right: -3.0 * UNIT_SIZE,
        top: 1.0 * UNIT_SIZE,
        bottom: 5.0 * UNIT_SIZE,
    };

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
