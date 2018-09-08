#![feature(nll)]
/*
TODO(JaSc):
  x Pixel perfect renderer 
    x Render to offscreen buffer and blit to main screen
    x Static world camera 
    x Transformation screen <-> canvas <-> world 
  x Atlas packer
  x Font packer
  x Atlas textures and sprite/quad/line-batching
  x Bitmap text rendering 
    x Worldspace/Screenspace placement
    x Depth clearing after switching from worldspace -> screenspace -> debugspace
    x Define and standardize fixed depth ranges for worldspace/screenspace/debugspace
  - Game input + keyboard/mouse-support
    - Change absolute/relative mouse position mode with system commands depending on being 
      in-menu/in-game
  - Gamestate + logic + timing
  - Audio playback
  - Some nice glowing shader effects
  - BG music with PHAT BEATSIES

TODO(JaSc): (Bigger things for vacations)
  x Throw out generalized coordinate system and replace by simple pixel-based coordinate system
  x Make framebuffer handling client side. For this we need to create some new draw commands and 
    restructure the platform layer a little
  - Make it possible for debug overlays like intersections to draw to world-space as well as 
    canvas-space to make i.e. arrow-heads uniformly sized regardless of arrow-size/zoom-level
  - Allow do draw lines with arbitrary thickness
  - Add system commands from client to platform that can change settings like vsync without 
    restart. This requires some major codeflow refactoring but would allow us to better modularize
    the platform layer. We also would need to re-upload all textures to the graphics context.

BACKLOG(JaSc):
  - The following are things to remember to extract out of the old C project in the long term
    x Debug macro to print a variable and it's name quickly
    x Be able to conveniently do debug printing on screen
    - Identification and sorting of translucent sprites
    - Moving camera system
    x Aseprite image parser and converter
    x Texture array of atlases implementation
    - Drawing debug overlays (grids/camera-frustums/crosshairs/depthbuffer)
    - Gamepad input
    x Correct mouse zooming and panning
    x Raycasting and collision detection
    x Fixed sized pixel perfect canvase (framebuffer)
    - Flexible sized pixel perfect canvase (framebuffer)
    - Live looped input playback and recording
    x Hot reloading of game code
    - Disable hot reloading when making a publish build
*/

extern crate game_lib;
extern crate libloading;
use game_lib::{GameContext, GameInput, Point, Rect, SystemCommand, Vec2};

mod game_interface;
mod graphics;
mod timer;

use game_interface::GameLib;
use graphics::{ColorFormat, DepthFormat, RenderingContext};
use timer::Timer;

extern crate failure;
use failure::{Error, ResultExt};

#[macro_use]
extern crate log;
extern crate fern;
extern crate rand;

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
use gfx::Device;
use glutin::GlContext;

pub trait OptionHelper {
    fn none_or(self, err: Error) -> Result<(), Error>;
}

impl<T> OptionHelper for Option<T> {
    fn none_or(self, err: Error) -> Result<(), Error> {
        match self {
            None => Ok(()),
            Some(_) => Err(err),
        }
    }
}

const LOG_LEVEL_GENERAL: log::LevelFilter = log::LevelFilter::Trace;
const LOG_LEVEL_MAIN: log::LevelFilter = log::LevelFilter::Info;
const LOG_LEVEL_GAME_INTERFACE: log::LevelFilter = log::LevelFilter::Info;
const LOG_LEVEL_GRAPHICS: log::LevelFilter = log::LevelFilter::Info;

//==================================================================================================
// Mainloop
//==================================================================================================
//
fn main() -> Result<(), Error> {
    // Initializing logger
    //
    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{}: {}", record.level(), message)))
        .level(LOG_LEVEL_GENERAL)
        .level_for("game_runtime", LOG_LEVEL_MAIN)
        .level_for("game_runtime::graphics", LOG_LEVEL_GRAPHICS)
        .level_for("game_runtime::game_interface", LOG_LEVEL_GAME_INTERFACE)
        .level_for("gfx_device_gl", log::LevelFilter::Warn)
        .level_for("winit", log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()
        .context("Could not initialize logger")?;

    // ---------------------------------------------------------------------------------------------
    // Video subsystem initialization
    //

    // TODO(JaSc): Read MONITOR_ID and FULLSCREEN_MODE from config file
    // TODO(JaSc): Once https://github.com/tomaka/winit/issues/574 is solved, fix windowed mode
    //             for relative mouse movement. For this we need not only check when we have focus
    //             but also only enable mouse grabbing/hiding when we click into the window content.
    //             If we don't do that we cannot click on 'x' or resize because our mouse will
    //             get dragged to the window center instantly.
    const MONITOR_ID: usize = 0;
    const FULLSCREEN_MODE: bool = true;
    const GL_VERSION_MAJOR: u8 = 3;
    const GL_VERSION_MINOR: u8 = 2;

    //
    info!("Getting monitor and its properties");
    //
    let mut events_loop = glutin::EventsLoop::new();
    let monitor = events_loop
        .get_available_monitors()
        .nth(MONITOR_ID)
        .ok_or_else(|| failure::err_msg(format!("No monitor with id {} found", MONITOR_ID)))?;

    let monitor_logical_dimensions = monitor
        .get_dimensions()
        .to_logical(monitor.get_hidpi_factor());

    info!(
        "Found monitor {} with logical dimensions: {:?}",
        MONITOR_ID,
        (
            monitor_logical_dimensions.width,
            monitor_logical_dimensions.height
        )
    );

    //
    info!("Creating window and drawing context");
    //
    let fullscreen_monitor = if FULLSCREEN_MODE { Some(monitor) } else { None };
    let window_builder = glutin::WindowBuilder::new()
        .with_resizable(!FULLSCREEN_MODE)
        // TODO(JaSc): Allow cursor grabbing in windowed mode when 
        //             https://github.com/tomaka/winit/issues/574
        //             is fixed. Grabbing the cursor in windowed mode and ALT-TABBING in and out
        //             is currently broken.
        .with_fullscreen(fullscreen_monitor)
        .with_title("Pongi".to_string());

    let context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::Specific(
            glutin::Api::OpenGl,
            (GL_VERSION_MAJOR, GL_VERSION_MINOR),
        ))
        // TODO(JaSc): Find out why CPU load is so high even though we use vsync
        .with_vsync(true);

    let (
        window,
        mut device,
        mut factory,
        screen_color_render_target_view,
        screen_depth_render_target_view,
    ) = gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &events_loop);

    let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut rc = RenderingContext::new(
        factory,
        encoder,
        screen_color_render_target_view,
        screen_depth_render_target_view,
    ).context("Could not create rendering context")?;

    // ---------------------------------------------------------------------------------------------
    // Main loop
    //

    // State variables
    let mut is_running = true;
    let mut mouse_pos_screen = Point::zero();
    let mut mouse_delta_screen = Vec2::zero();
    let mut screen_dimensions = Vec2::zero();
    let mut ready_to_modify_cursor = false;
    let mut window_has_focus = true;
    let mut relative_mouse_mode_enabled = false;

    let mut input = GameInput::new();
    input.do_reinit_gamestate = true;
    input.do_reinit_drawstate = true;
    input.hotreload_happened = true;
    input.game_paused = true;

    let mut game_lib = GameLib::new("target/debug/", "game_interface_glue");
    let mut game_context = GameContext::new();

    let timer_startup = Timer::new();
    let mut timer_delta = Timer::new();
    //
    info!("Entering main event loop");
    info!("------------------------");
    //
    while is_running {
        // Testing library hotreloading
        if game_lib.needs_reloading() {
            game_lib = game_lib.reload();
            if !game_lib.needs_reloading() {
                // The game actually reloaded
                input.hotreload_happened = true;
            }
        }

        use glutin::{Event, KeyboardInput, WindowEvent};
        events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => is_running = false,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(key),
                                // modifiers,
                                ..
                            },
                        ..
                    } => {
                        use glutin::VirtualKeyCode::*;
                        match key {
                            Escape => is_running = false,
                            F1 => input.do_reinit_gamestate = true,
                            F5 => input.do_reinit_drawstate = true,
                            F9 => {
                                input.direct_screen_drawing = !input.direct_screen_drawing;
                                input.do_reinit_drawstate = true;
                            }
                            Add => input.fast_time += 1,
                            Subtract => input.fast_time -= 1,
                            Space => input.game_paused = !input.game_paused,
                            _ => (),
                        }
                    }
                    WindowEvent::Focused(has_focus) => {
                        info!("Window has focus: {}", has_focus);
                        window_has_focus = has_focus;
                        // NOTE: We need to grab/ungrab and hide/unhide mouse cursor when
                        //       ALT-TABBING in and out or the user cannot use their computer
                        //       correctly in a multi-monitor setup while running our app.
                        if ready_to_modify_cursor {
                            if FULLSCREEN_MODE {
                                window.grab_cursor(has_focus).unwrap();
                            }
                            if relative_mouse_mode_enabled {
                                window.hide_cursor(has_focus);
                            }
                        }
                    }
                    WindowEvent::Resized(new_dim) => {
                        window.resize(new_dim.to_physical(window.get_hidpi_factor()));
                        gfx_window_glutin::update_views(
                            &window,
                            &mut rc.screen_framebuffer.color_render_target_view,
                            &mut rc.screen_framebuffer.depth_render_target_view,
                        );
                        rc.update_screen_dimensions(new_dim.width as u16, new_dim.height as u16);
                        screen_dimensions = Vec2::new(new_dim.width as f32, new_dim.height as f32);

                        // Grab and/or hide mouse cursor in window
                        // NOTE: Due to https://github.com/tomaka/winit/issues/574 we need to first
                        //       make sure that our resized window now spans the full screen before
                        //       we allow grabbing the mouse cursor.
                        // TODO(JaSc): Remove workaround when upstream is fixed
                        if FULLSCREEN_MODE && new_dim == monitor_logical_dimensions {
                            // Our window now has its final size, we can safely grab the cursor now
                            info!("Mouse cursor grabbed");
                            ready_to_modify_cursor = true;
                            window.grab_cursor(true).unwrap();
                        }
                        if relative_mouse_mode_enabled && ready_to_modify_cursor {
                            window.hide_cursor(true);
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        // NOTE: mouse_pos_screen is in the following interval:
                        //       [0 .. screen_width - 1] x [0 .. screen_height - 1]
                        //       where (0,0) is the top left of the screen
                        let pos = Point::new(position.x as f32, position.y as f32);
                        if relative_mouse_mode_enabled {
                            // NOTE: We do not use '+=' as we only want to save the last delta
                            //       that we registered during the last frame.
                            mouse_delta_screen = pos - screen_dimensions / 2.0;
                        } else {
                            mouse_delta_screen += pos - mouse_pos_screen;
                            mouse_pos_screen = pos;
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        input.mouse_wheel_delta += match delta {
                            glutin::MouseScrollDelta::LineDelta(_, y) => y as i32,
                            glutin::MouseScrollDelta::PixelDelta(pos) => pos.y as i32,
                        };
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        use glutin::ElementState;
                        use glutin::MouseButton;

                        let is_pressed = match state {
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        };
                        match button {
                            MouseButton::Left => input.mouse_button_left.set_state(is_pressed),
                            MouseButton::Middle => input.mouse_button_middle.set_state(is_pressed),
                            MouseButton::Right => input.mouse_button_right.set_state(is_pressed),
                            _ => {}
                        }
                    }
                    _ => (),
                }
            }
        });

        if relative_mouse_mode_enabled && window_has_focus {
            mouse_pos_screen += mouse_delta_screen;
            mouse_pos_screen = mouse_pos_screen.clamped_in_rect(Rect::from_width_height(
                screen_dimensions.x - 1.0,
                screen_dimensions.y - 1.0,
            ));

            // TODO(JaSc): Maybe we need to set this more frequently?
            window
                .set_cursor_position(glutin::dpi::LogicalPosition::new(
                    f64::from(screen_dimensions.x) / 2.0,
                    f64::from(screen_dimensions.y) / 2.0,
                ))
                .unwrap();
        }

        // Prepare input and update game
        input.mouse_pos_screen = mouse_pos_screen;
        input.mouse_delta_screen = mouse_delta_screen;
        mouse_delta_screen = Vec2::zero();

        input.screen_dim = screen_dimensions;
        input.time_since_startup = timer_startup.elapsed_time();
        input.time_delta = timer_delta.elapsed_time() as f32;
        timer_delta.reset();

        let timer_update = Timer::new();
        game_lib.update_and_draw(&input, &mut game_context);
        input.time_update = timer_update.elapsed_time() as f32;

        // Process Systemcommands
        for command in game_context.get_system_commands() {
            match command {
                SystemCommand::EnableRelativeMouseMovementCapture(do_enable) => {
                    if ready_to_modify_cursor {
                        window.hide_cursor(do_enable && window_has_focus);
                    } else {
                        unimplemented!();
                        // TODO(JaSc): We need to remember to hide the cursor once we can modify it.
                        //             Or we could just wait for
                        //             https://github.com/tomaka/winit/issues/574 to be fixed
                        //             and just drop the 'ready_to_modify_cursor' concept altogether
                    }
                    relative_mouse_mode_enabled = do_enable;
                }
            }
        }

        // Draw to screen
        let timer_draw = Timer::new();
        rc.process_draw_commands(game_context.get_draw_commands())
            .context("Could not to process a draw command")?;
        input.time_draw = timer_draw.elapsed_time() as f32;

        // Flush and flip buffers
        rc.encoder.flush(&mut device);
        window
            .swap_buffers()
            .context("Could not to swap framebuffers")?;
        device.cleanup();

        // Reset input
        input.clear_button_transitions();
        input.clear_flags();
    }

    Ok(())
}
