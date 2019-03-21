pub use crate::gui::{GuiContext, ScreenFader};
use crate::*;

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

#[derive(Debug, Clone, Copy)]
enum GameDifficulty {
    Easy,
    Medium,
    Hard,
}

impl Default for GameDifficulty {
    fn default() -> Self {
        GameDifficulty::Medium
    }
}

#[derive(Default)]
pub struct Globals {
    pub restart_game: bool,
    pub input_disabled: bool,

    pub debug_time_factor_increment: i32,
    pub debug_game_paused: bool,
    pub game_paused: bool,

    game_difficulty: GameDifficulty,
    right_player_is_human: bool,
    left_player_is_human: bool,

    pub mouse_pos_world: WorldPoint,
    pub mouse_pos_canvas: CanvasPoint,

    pub mouse_delta_world: WorldVec,
    pub mouse_delta_canvas: CanvasVec,

    pub cam: Camera,
    pub error_happened: Option<String>,
}

// The Scene system is heavily inspired by ggez and amethyst
pub trait Scene {
    fn reinitialize(&mut self, system_commands: &mut Vec<SystemCommand>);
    fn update_and_draw(
        &mut self,
        input: &GameInput,
        globals: &mut Globals,
        dc: &mut DrawContext,
        ac: &mut AudioContext,
        system_commands: &mut Vec<SystemCommand>,
    );
    fn update_and_draw_previous_scene(&self) -> bool {
        false
    }
}

//==================================================================================================
// DebugScene
//==================================================================================================
//
#[derive(Default)]
pub struct DebugScene;

impl Scene for DebugScene {
    fn reinitialize(&mut self, _system_commands: &mut Vec<SystemCommand>) {}
    fn update_and_draw(
        &mut self,
        input: &GameInput,
        globals: &mut Globals,
        dc: &mut DrawContext,
        ac: &mut AudioContext,
        _system_commands: &mut Vec<SystemCommand>,
    ) {
        if input.had_press_event("debug_play_sound") {
            ac.play_debug_sound(audio::SoundStartTime::OnNextBeat);
        }

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
        // dc.debug_draw_cursor(
        //     globals.mouse_pos_canvas,
        //     -0.3,
        //     COLOR_WHITE,
        //     DrawSpace::Canvas,
        // );
        dc.draw_rect_filled(
            Rect::from_point_dimension(globals.mouse_pos_world.pixel_snapped(), Vec2::ones()),
            -0.2,
            cursor_color,
            0.0,
            DrawSpace::World,
        );

        // Frametimes etc.
        let delta = pretty_format_duration_ms(f64::from(input.time_delta));
        let draw = pretty_format_duration_ms(f64::from(input.time_draw));
        let update = pretty_format_duration_ms(f64::from(input.time_update));
        let audio = pretty_format_duration_ms(f64::from(input.time_audio));
        dc.debug_draw_text(
            &format!(
                "delta: {}\ndraw: {}\nupdate: {}\naudio: {}\n",
                delta, draw, update, audio
            ),
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
                input.mouse_delta_screen.x, input.mouse_delta_screen.y
            ),
            draw::COLOR_WHITE,
        );
        dc.debug_draw_text(
            &format!(
                "mouse_world: {}x{}",
                globals.mouse_pos_world.x, globals.mouse_pos_world.y
            ),
            draw::COLOR_WHITE,
        );
        dc.debug_draw_text(
            &format!(
                "mouse_canvas: {}x{}\n",
                globals.mouse_pos_canvas.x, globals.mouse_pos_canvas.y
            ),
            draw::COLOR_WHITE,
        );

        if globals.debug_time_factor_increment != 0 {
            if globals.debug_time_factor_increment > 0 {
                dc.debug_draw_text(
                    &format!("Time speedup {}x", globals.debug_time_factor_increment + 1),
                    draw::COLOR_GREEN,
                );
            } else if globals.debug_time_factor_increment < 0 {
                dc.debug_draw_text(
                    &format!(
                        "Time slowdown {}x",
                        i32::abs(globals.debug_time_factor_increment) + 1
                    ),
                    draw::COLOR_YELLOW,
                );
            } else {
            };
        }
        if globals.game_paused {
            dc.debug_draw_text("The game is paused", draw::COLOR_CYAN);
        }
        // Debug crash message
        if globals.error_happened.is_some() {
            dc.debug_draw_text(
                &format!(
                    "The game has crashed: {}",
                    globals.error_happened.clone().unwrap()
                ),
                draw::COLOR_RED,
            );
        }
    }
}

//==================================================================================================
// GameplayScene
//==================================================================================================
//
#[derive(Default)]
pub struct GameplayScene {
    is_paused: bool,

    paddle_left_pos: f32,
    paddle_left_vel: f32,

    paddle_right_pos: f32,
    paddle_right_vel: f32,

    pongi_pos: WorldPoint,
    pongi_vel: Vec2,

    time_till_next_beat: f32,

    game_difficulty: GameDifficulty,
    right_player_is_human: bool,
    left_player_is_human: bool,
}

impl Scene for GameplayScene {
    fn reinitialize(&mut self, _system_commands: &mut Vec<SystemCommand>) {
        self.is_paused = false;
        //gc.pongi_pos = Point::new(0.0, -3.0 * UNIT_SIZE);
        //gc.pongi_vel = Vec2::new(0.0, -5.0 * UNIT_SIZE);

        let angle: f32 = 40.0;
        self.pongi_pos = Point::new(8.0, -4.0) * UNIT_SIZE;
        self.pongi_vel = Vec2::from_angle(angle.to_radians()) * PONGI_BASE_SPEED;

        // gc.pongi_pos = Point::new(-151.48575, -88.0);
        // gc.pongi_vel = Vec2::new(-4644.807, 6393.034);
    }

    fn update_and_draw(
        &mut self,
        input: &GameInput,
        globals: &mut Globals,
        dc: &mut DrawContext,
        _ac: &mut AudioContext,
        system_commands: &mut Vec<SystemCommand>,
    ) {
        if globals.restart_game {
            globals.restart_game = false;
            self.left_player_is_human = globals.left_player_is_human;
            self.right_player_is_human = globals.right_player_is_human;
            self.game_difficulty = globals.game_difficulty;
            self.reinitialize(system_commands);
        }

        let delta_time =
            if self.is_paused || globals.debug_game_paused || globals.error_happened.is_some() {
                0.0
            } else {
                let time_factor = if globals.debug_time_factor_increment == 0 {
                    1.0
                } else if globals.debug_time_factor_increment > 0 {
                    (globals.debug_time_factor_increment + 1) as f32
                } else {
                    1.0 / (((i32::abs(globals.debug_time_factor_increment)) + 1) as f32)
                };
                input.time_delta * time_factor
            };

        // ---------------------------------------------------------------------------------------------
        // Playfield
        //

        let screen_rect = Rect::from_dimension(input.screen_dim);
        let canvas_rect = Rect::from_width_height(CANVAS_WIDTH, CANVAS_HEIGHT);

        // Draw grid
        let grid_light = Color::new(0.9, 0.7, 0.2, 1.0);
        for x in -30..30 {
            for diagonal in -20..20 {
                let pos = Point::new((x + diagonal) as f32, diagonal as f32) * UNIT_SIZE;
                if x % 2 == 0 {
                    dc.draw_rect_filled(
                        Rect::from_point_dimension(pos, Vec2::ones() * UNIT_SIZE),
                        -1.0,
                        grid_light,
                        ADDITIVITY_NONE,
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
                        ADDITIVITY_NONE,
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
            dc.draw_rect_filled(
                field_border,
                field_depth,
                color,
                ADDITIVITY_NONE,
                DrawSpace::World,
            );
        }

        // Update beat
        const BPM: f32 = 100.0;
        const BEAT_LENGTH: f32 = 60.0 / BPM;

        let mut time_till_next_beat = self.time_till_next_beat;
        time_till_next_beat -= delta_time;
        while time_till_next_beat < 0.0 {
            time_till_next_beat += BEAT_LENGTH;
        }
        let beat_value = beat_visualizer_value(time_till_next_beat, BEAT_LENGTH);

        // Update pongi
        let pongi_pos = self.pongi_pos;
        let pongi_vel = self.pongi_vel;

        let mut collision_mesh = CollisionMesh::new("play_field");
        collision_mesh.add_rect("left_wall", field_border_left);
        collision_mesh.add_rect("right_wall", field_border_right);
        collision_mesh.add_rect("top_wall", field_border_top);
        collision_mesh.add_rect("bottom_wall", field_border_bottom);
        //collision_mesh.add_rect("center_wall", field_border_center);

        let mut error_happened = None;
        let (new_pongi_pos, new_pongi_vel) = move_sphere_with_full_elastic_collision(
            &collision_mesh,
            pongi_pos,
            pongi_vel,
            PONGI_RADIUS,
            delta_time,
        ).unwrap_or_else(|error| {
            error_happened = Some(error);
            (pongi_pos, pongi_vel)
        });

        // Write back to game_context
        globals.error_happened = error_happened;
        if globals.error_happened.is_none() {
            self.pongi_vel = new_pongi_vel;
            self.pongi_pos = new_pongi_pos;
            self.time_till_next_beat = time_till_next_beat;
            self.paddle_left_pos = clamp(
                self.paddle_left_pos
                    + input.mouse_delta_screen.y / screen_rect.height() * canvas_rect.height(),
                FIELD_BOUNDS.top,
                FIELD_BOUNDS.bottom - PADDLE_SIZE,
            );
        }

        // Debug draw sphere sweeping
        collision_mesh
            .shapes
            .iter()
            .map(|rect| RectSphereSum::new(rect, PONGI_RADIUS))
            .for_each(|sum| {
                dc.draw_lines(
                    &sum.to_lines(),
                    0.0,
                    COLOR_YELLOW,
                    ADDITIVITY_NONE,
                    DrawSpace::World,
                )
            });

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
            ADDITIVITY_NONE,
            DrawSpace::Canvas,
        );

        // Draw pongi
        dc.debug_draw_text(&dformat!(self.pongi_vel), draw::COLOR_WHITE);
        dc.debug_draw_text(&dformat!(self.pongi_pos), draw::COLOR_WHITE);
        dc.draw_arrow(
            self.pongi_pos.pixel_snapped(),
            self.pongi_vel.normalized(),
            0.3 * self.pongi_vel.magnitude(),
            -0.1,
            draw::COLOR_GREEN,
            ADDITIVITY_NONE,
            DrawSpace::World,
        );

        dc.debug_draw_circle_textured(
            self.pongi_pos.pixel_snapped(),
            -0.3,
            Color::new(1.0 - beat_value, 1.0 - beat_value, 1.0, 1.0),
            ADDITIVITY_NONE,
            DrawSpace::World,
        );

        // Draw paddles
        dc.draw_rect_filled(
            Rect::from_point(
                WorldPoint::new(FIELD_BOUNDS.left - WALL_THICKNESS, self.paddle_left_pos),
                WALL_THICKNESS,
                PADDLE_SIZE,
            ),
            -0.2,
            COLOR_WHITE,
            ADDITIVITY_NONE,
            DrawSpace::World,
        );
        dc.draw_rect_filled(
            Rect::from_point(
                WorldPoint::new(FIELD_BOUNDS.right, self.paddle_right_pos),
                WALL_THICKNESS,
                PADDLE_SIZE,
            ),
            -0.2,
            COLOR_WHITE,
            ADDITIVITY_NONE,
            DrawSpace::World,
        );
    }
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

//==================================================================================================
// MenuScene
//==================================================================================================
//

#[derive(Debug, Clone, Copy, PartialEq)]
enum MenuMode {
    Ingame,
    Main,
    Difficulty,
    Pause,
}

impl Default for MenuMode {
    fn default() -> Self {
        MenuMode::Main
    }
}

#[derive(Debug, Clone, Copy)]
enum MenuItem {
    MainStartSinglePlayer,
    MainStartTwoPlayers,
    MainQuit,
    DifficultyEasy,
    DifficultyMedium,
    DifficultyHard,
    DifficultyBack,
    PauseResume,
    PauseQuitMenu,
}

impl MenuItem {
    pub fn as_str(&self) -> &'static str {
        match self {
            MenuItem::MainStartSinglePlayer => "Play with computer",
            MenuItem::MainStartTwoPlayers => "Play with human",
            MenuItem::MainQuit => "Quit game",
            MenuItem::DifficultyEasy => "Easy",
            MenuItem::DifficultyMedium => "Medium",
            MenuItem::DifficultyHard => "Hard",
            MenuItem::DifficultyBack => "Back",
            MenuItem::PauseResume => "Resume",
            MenuItem::PauseQuitMenu => "Quit to menu",
        }
    }
}

const MAIN_MENU_ITEMS: &[MenuItem] = &[
    MenuItem::MainStartSinglePlayer,
    MenuItem::MainStartTwoPlayers,
    MenuItem::MainQuit,
];

const DIFFICULTY_MENU_ITEMS: &[MenuItem] = &[
    MenuItem::DifficultyEasy,
    MenuItem::DifficultyMedium,
    MenuItem::DifficultyHard,
    MenuItem::DifficultyBack,
];

const PAUSE_MENU_ITEMS: &[MenuItem] = &[MenuItem::PauseResume, MenuItem::PauseQuitMenu];

const FADE_TIME: f32 = 0.2;

#[derive(Debug, Default)]
pub struct MenuScene {
    menu_mode: MenuMode,
    screen_fader: ScreenFader,
    gui: GuiContext,
}

impl Scene for MenuScene {
    fn reinitialize(&mut self, system_commands: &mut Vec<SystemCommand>) {
        system_commands.push(SystemCommand::EnableRelativeMouseMovementCapture(false));
    }

    fn update_and_draw(
        &mut self,
        input: &GameInput,
        globals: &mut Globals,
        dc: &mut DrawContext,
        _ac: &mut AudioContext,
        system_commands: &mut Vec<SystemCommand>,
    ) {
        let canvas_rect = Rect::from_width_height(CANVAS_WIDTH, CANVAS_HEIGHT);

        // Update screen fader
        self.screen_fader.increment(input.time_delta);
        if self.screen_fader.has_finished_fading_out() {
            self.menu_mode = match self.menu_mode {
                MenuMode::Ingame => MenuMode::Ingame,
                MenuMode::Main => MenuMode::Ingame,
                MenuMode::Difficulty => MenuMode::Ingame,
                MenuMode::Pause => MenuMode::Main,
            };
            globals.restart_game = true;
            self.screen_fader.start_fading_in(FADE_TIME);
        }
        if self.screen_fader.has_finished_fading_in() {
            // TODO(JaSc): Right now this is excecuted nearly every frame. Maybe those fader
            //             'has_finished_fading_..' methods should only return true once.
            //             Alternatively we could pass closures to the increment method that
            //             will get executed when screen_fader finished fading
            globals.input_disabled = false;
        }

        // Fade overlay
        if self.screen_fader.fading_overlay_opacity() > 0.0 {
            dc.draw_rect_filled(
                canvas_rect,
                0.0,
                Color::new(1.0, 1.0, 1.0, self.screen_fader.fading_overlay_opacity()),
                ADDITIVITY_NONE,
                // TODO(JaSc): For now we just use the debug-drawspace as a hack until we implement
                //             transparency sorting. If we would draw this in canvas-drawspace
                //             it would interfere with menu overlay.
                DrawSpace::Debug,
            );
        }

        // Enable or disable relative mouse movement capture
        if self.menu_mode == MenuMode::Ingame {
            if input.had_press_event("ui_escape") {
                system_commands.push(SystemCommand::EnableRelativeMouseMovementCapture(false));
                self.menu_mode = MenuMode::Pause;
                // NOTE: We return here immediately so we can start fresh in the pause menu next
                //       time we call this method. Otherwise the escape key press will be evaluated
                //       again later in this method which causes some issues.
                return;
            } else {
                // No need to show a menu if we are in-game
                system_commands.push(SystemCommand::EnableRelativeMouseMovementCapture(true));
                return;
            }
        }

        // Overlay below scene
        dc.draw_rect_filled(
            canvas_rect,
            -0.2,
            Color::new(1.0, 1.0, 1.0, 0.3),
            ADDITIVITY_NONE,
            DrawSpace::Canvas,
        );

        // Create menu
        let menu_items = match self.menu_mode {
            MenuMode::Ingame => &[],
            MenuMode::Main => MAIN_MENU_ITEMS,
            MenuMode::Difficulty => DIFFICULTY_MENU_ITEMS,
            MenuMode::Pause => PAUSE_MENU_ITEMS,
        };
        let menu_items_strings = menu_items
            .iter()
            .map(|item| item.as_str())
            .collect::<Vec<_>>();

        let mut clicked_menu_item = create_button_menu(
            &menu_items_strings,
            &mut self.gui,
            -0.1,
            canvas_rect,
            input,
            globals,
            dc,
        ).map(|index| menu_items[index]);

        // Override clicked_menu_item if we pressed escape this frame
        if input.had_press_event("ui_escape") {
            if self.menu_mode == MenuMode::Pause {
                clicked_menu_item = Some(MenuItem::PauseQuitMenu);
            } else if self.menu_mode == MenuMode::Main {
                clicked_menu_item = Some(MenuItem::MainQuit);
            }
        }

        // Evaluate the clicked_menu_item
        if let Some(clicked_menu_item) = clicked_menu_item {
            if !globals.input_disabled {
                match clicked_menu_item {
                    MenuItem::MainStartSinglePlayer => self.menu_mode = MenuMode::Difficulty,
                    MenuItem::MainStartTwoPlayers => {
                        globals.input_disabled = true;
                        globals.game_difficulty = GameDifficulty::Medium;
                        globals.left_player_is_human = true;
                        globals.right_player_is_human = true;
                        self.screen_fader.start_fading_out(FADE_TIME);
                    }
                    MenuItem::MainQuit => system_commands.push(SystemCommand::ShutdownGame),
                    MenuItem::DifficultyEasy => {
                        globals.input_disabled = true;
                        globals.game_difficulty = GameDifficulty::Easy;
                        globals.left_player_is_human = true;
                        globals.right_player_is_human = false;
                        self.screen_fader.start_fading_out(FADE_TIME);
                    }
                    MenuItem::DifficultyMedium => {
                        globals.input_disabled = true;
                        globals.game_difficulty = GameDifficulty::Medium;
                        globals.left_player_is_human = true;
                        globals.right_player_is_human = false;
                        self.screen_fader.start_fading_out(FADE_TIME);
                    }
                    MenuItem::DifficultyHard => {
                        globals.input_disabled = true;
                        globals.game_difficulty = GameDifficulty::Hard;
                        globals.left_player_is_human = true;
                        globals.right_player_is_human = false;
                        self.screen_fader.start_fading_out(FADE_TIME);
                    }
                    MenuItem::DifficultyBack => self.menu_mode = MenuMode::Main,
                    MenuItem::PauseResume => {
                        self.menu_mode = MenuMode::Ingame;
                    }
                    MenuItem::PauseQuitMenu => {
                        globals.input_disabled = true;
                        globals.game_difficulty = GameDifficulty::Medium;
                        globals.left_player_is_human = false;
                        globals.right_player_is_human = false;
                        self.screen_fader.start_fading_out(FADE_TIME);
                    }
                }
            }
        }
    }
}

fn create_button_menu(
    menu_items: &[&str],
    gui: &mut GuiContext,
    depth: f32,
    canvas_rect: Rect,
    input: &GameInput,
    globals: &mut Globals,
    dc: &mut DrawContext,
) -> Option<usize> {
    if menu_items.len() == 0 {
        return None;
    }

    // Create button sizes
    let button_margin = 1.0;
    let button_padding = 4.0;
    let button_dim = menu_items
        .iter()
        .map(|item| dc.get_text_dimensions(item) + 2.0 * button_padding)
        .fold(Rect::zero(), |acc, dim| {
            Rect::smallest_rect_that_contains_both_rects(acc, Rect::from_dimension(dim))
        })
        .dim();

    // Create and draw menu box
    let menu_padding = 2.0;
    let menu_height = 2.0 * menu_padding
        + menu_items.len() as f32 * button_dim.y
        + ((menu_items.len() - 1) as f32) * button_margin;
    let menu_width = button_dim.x + 2.0 * menu_padding;
    let menu_box = Rect::from_width_height(menu_width, menu_height)
        .centered_in_rect(canvas_rect)
        .with_pixel_snapped_position();
    dc.draw_rect_filled(menu_box, depth, COLOR_CYAN, 0.0, DrawSpace::Canvas);
    dc.draw_rect(
        menu_box,
        depth,
        Color::new(0.4, 0.4, 0.4, 0.4),
        ADDITIVITY_NONE,
        DrawSpace::Canvas,
    );

    // Create and draw buttons
    gui.start(globals.mouse_pos_canvas, &input);
    let mut clicked_button_index = None;
    let mut vertical_offset = menu_padding;

    for (index, label) in menu_items.iter().enumerate() {
        // Draw button rect
        let button_rect = Rect::from_dimension(button_dim)
            .translated_to_pos(menu_box.pos())
            .translated_by(Vec2::unit_y() * vertical_offset)
            .centered_horizontally_in_rect(menu_box)
            .with_pixel_snapped_position();
        vertical_offset += button_rect.height() + button_margin;

        if gui.button(index, label, button_rect, depth, dc) {
            clicked_button_index = Some(index);
        }
    }
    gui.finish();

    clicked_button_index
}
