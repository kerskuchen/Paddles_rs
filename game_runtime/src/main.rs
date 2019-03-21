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
  x Game input + keyboard/mouse-support
    x Change absolute/relative mouse position mode with system commands depending on being
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
  - Remove gfx-rs as it is too overkill for our purposes
  - Move 'Keycode' into game_lib and pass complete input from the platform layer into game_lib
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

extern crate libloading;
use game_lib::{self, GameContext, GameInput, Point, Rect, SystemCommand, Vec2};

mod game_interface;
mod graphics;
mod input;
mod timer;

use crate::game_interface::GameLib;
use crate::graphics::{ColorFormat, DepthFormat, RenderingContext};
use crate::timer::Timer;

use failure::{self, Error, ResultExt};

use cpal;

use fern;
use log::*;

use gfx;
use gfx_window_sdl;
use sdl2;

use gfx::Device;

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
    const FULLSCREEN_MODE: bool = false;
    const GL_VERSION_MAJOR: u8 = 3;
    const GL_VERSION_MINOR: u8 = 2;

    let sdl_context = sdl2::init().expect("Could not initialize SDL2");
    let mut events = sdl_context
        .event_pump()
        .expect("Could not retrieve SDL2 event pump");
    let video_subsystem = sdl_context
        .video()
        .expect("Could init SDL2 video subsystem");

    // TODO: Re-enable this
    let screen_width = 1024;
    let screen_heigth = 768;
    //    //
    //    info!("Getting monitor and its properties");
    //    //
    //    let mut events_loop = glutin::EventsLoop::new();
    //    let monitor = events_loop
    //        .get_available_monitors()
    //        .nth(MONITOR_ID)
    //        .ok_or_else(|| failure::err_msg(format!("No monitor with id {} found", MONITOR_ID)))?;
    //
    //    let monitor_logical_dimensions = monitor
    //        .get_dimensions()
    //        .to_logical(monitor.get_hidpi_factor());
    //
    //    info!(
    //        "Found monitor {} with logical dimensions: {:?}",
    //        MONITOR_ID,
    //        (
    //            monitor_logical_dimensions.width,
    //            monitor_logical_dimensions.height
    //        )
    //    );

    //
    info!("Creating window and drawing context");
    //
    // Configure OpenGl
    let mut window_builder = video_subsystem.window("Paddles", screen_width, screen_heigth);

    if FULLSCREEN_MODE {
        window_builder.fullscreen_desktop().input_grabbed();
    } else {
        window_builder.resizable().position_centered();
    }

    let (
        window,
        _gl_context,
        mut device,
        mut factory,
        screen_color_render_target_view,
        screen_depth_render_target_view,
    ) = gfx_window_sdl::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(
        &video_subsystem,
        window_builder,
    )
    .unwrap();

    video_subsystem
        .gl_attr()
        .set_context_profile(sdl2::video::GLProfile::Core);
    video_subsystem.gl_attr().set_context_version(3, 2);
    video_subsystem
        .gl_set_swap_interval(sdl2::video::SwapInterval::VSync)
        .unwrap();

    let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut rc = RenderingContext::new(
        factory,
        encoder,
        screen_color_render_target_view,
        screen_depth_render_target_view,
    )
    .context("Could not create rendering context")?;

    gfx_window_sdl::update_views(
        &window,
        &mut rc.screen_framebuffer.color_render_target_view,
        &mut rc.screen_framebuffer.depth_render_target_view,
    );
    rc.update_screen_dimensions(screen_width as u16, screen_heigth as u16);
    let mut screen_dimensions = Vec2::new(screen_width as f32, screen_heigth as f32);

    info!("4 window and drawing context");
    // ---------------------------------------------------------------------------------------------
    // Audio subsystem initialization
    //

    // TODO: Audio

    //    // Init device and audio stream
    //    let audio_device = cpal::default_output_device()
    //        .ok_or_else(|| failure::err_msg("Could not create audio output device"))?;
    //    let audio_format = audio_device
    //        .default_output_format()
    //        .context("Could not get audio devices default ouput format")?;
    //
    //    let audio_event_loop = cpal::EventLoop::new();
    //    let audio_stream = audio_event_loop
    //        .build_output_stream(&audio_device, &audio_format)
    //        .context("Could not create audio output stream")?;
    //    audio_event_loop.play_stream(audio_stream);
    //
    //    let audio_sample_rate = audio_format.sample_rate.0 as usize;
    //    let num_audio_channels = audio_format.channels as usize;
    //    info!(
    //        "Initialized audio_device {:?} with output format {:?}",
    //        audio_device.name(),
    //        audio_format
    //    );
    //
    //    use std::sync::{Arc, Mutex};
    //    let audio_output_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    //    let audio_output_buffer_clone = Arc::clone(&audio_output_buffer);
    //
    //    // Audio stream thread
    //    use std::ops::DerefMut;
    //    std::thread::spawn(move || {
    //        audio_event_loop.run(move |_, data| {
    //            let mut samples = audio_output_buffer_clone.lock().unwrap();
    //
    //            let (num_samples_committed, num_samples_required) = match data {
    //                cpal::StreamData::Output {
    //                    buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer),
    //                } => {
    //                    let mut num_samples_committed = 0;
    //                    for (output, value) in buffer.iter_mut().zip(samples.iter()) {
    //                        *output = *value;
    //                        num_samples_committed += 1;
    //                    }
    //                    let num_samples_required = buffer.deref_mut().len();
    //                    (num_samples_committed, num_samples_required)
    //                }
    //                cpal::StreamData::Output {
    //                    buffer: cpal::UnknownTypeOutputBuffer::U16(mut buffer),
    //                } => {
    //                    let mut num_samples_committed = 0;
    //                    for (output, value) in buffer.iter_mut().zip(samples.iter()) {
    //                        let value = ((*value * 0.5 + 0.5) * std::u16::MAX as f32) as u16;
    //                        *output = value;
    //                        num_samples_committed += 1;
    //                    }
    //                    let num_samples_required = buffer.deref_mut().len();
    //                    (num_samples_committed, num_samples_required)
    //                }
    //                cpal::StreamData::Output {
    //                    buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer),
    //                } => {
    //                    let mut num_samples_committed = 0;
    //                    for (output, value) in buffer.iter_mut().zip(samples.iter()) {
    //                        let value = (value * std::i16::MAX as f32) as i16;
    //                        *output = value;
    //                        num_samples_committed += 1;
    //                    }
    //                    let num_samples_required = buffer.deref_mut().len();
    //                    (num_samples_committed, num_samples_required)
    //                }
    //                _ => (0, 0),
    //            };
    //
    //            if num_samples_committed < num_samples_required {
    //                warn!(
    //                    "Audio lagged behind and skipped {} samples",
    //                    num_samples_required - num_samples_committed
    //                );
    //                samples.clear();
    //            } else {
    //                samples.drain(0..num_samples_committed);
    //            }
    //        });
    //    });

    // ---------------------------------------------------------------------------------------------
    // Main loop
    //

    // State variables
    let mut is_running = true;
    let mut mouse_pos_screen = Point::zero();
    let mut mouse_delta_screen = Vec2::zero();
    let mut window_has_focus = true;
    let mut relative_mouse_mode_enabled = false;

    // Init keymappings and gamebuttons for input
    let mut input = GameInput::new();
    let key_mapping = {
        let mut key_mapping = game_lib::utility::deserialize_from_ron_file::<input::Keymapping>(
            "data/key_mapping.txt",
        )
        .key_mapping;

        // Add debug keymapping if it exist
        if std::path::Path::new("data/key_mapping_debug.txt").exists() {
            let debug_key_mapping =
                game_lib::utility::deserialize_from_ron_file::<input::Keymapping>(
                    "data/key_mapping_debug.txt",
                )
                .key_mapping;
            key_mapping.extend(debug_key_mapping);
        }

        // Create buttons for input actions
        for actions in key_mapping.values() {
            for action in actions {
                input.register_input_action(action);
            }
        }

        key_mapping
    };

    // Gamelib loading and timing
    let mut game_lib = GameLib::new("target/debug/", "game_lib");
    // TODO: Audio
    //let mut game_context = GameContext::new(num_audio_channels, audio_sample_rate);
    let mut game_context = GameContext::new(2, 48000);

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
                input.process_button_event("debug_hotreload_code_oneshot", true);
            }
        }

        use sdl2::event::Event;
        use sdl2::event::WindowEvent;
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => is_running = false,
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    if let Some(input_actions) =
                        key_mapping.get(&input::convert_sdl_keycode_to_our_format(keycode))
                    {
                        for action in input_actions {
                            input.process_button_event(&action, true);
                        }
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    if let Some(input_actions) =
                        key_mapping.get(&input::convert_sdl_keycode_to_our_format(keycode))
                    {
                        for action in input_actions {
                            input.process_button_event(&action, false);
                        }
                    }
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::FocusGained => {
                        info!("Window gained focus");
                        window_has_focus = true;
                    }
                    WindowEvent::FocusLost => {
                        info!("Window lost focus");
                        window_has_focus = false;
                    }
                    WindowEvent::Resized(width, height) => {
                        info!("Window resized: {}x{}", width, height);
                        gfx_window_sdl::update_views(
                            &window,
                            &mut rc.screen_framebuffer.color_render_target_view,
                            &mut rc.screen_framebuffer.depth_render_target_view,
                        );
                        rc.update_screen_dimensions(width as u16, height as u16);
                        screen_dimensions = Vec2::new(width as f32, height as f32);
                    }
                    _ => {}
                },
                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    // NOTE: mouse_pos_screen is in the following interval:
                    //       [0 .. screen_width - 1] x [0 .. screen_height - 1]
                    //       where (0,0) is the top left of the screen
                    mouse_pos_screen = Point::new(x as f32, y as f32);
                    mouse_delta_screen = Vec2::new(xrel as f32, yrel as f32);
                }
                Event::MouseWheel { y, .. } => {
                    input.mouse_wheel_delta += y;
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    let is_pressed = true;
                    use sdl2::mouse::MouseButton;
                    match mouse_btn {
                        MouseButton::Left => input.mouse_button_left.set_state(is_pressed),
                        MouseButton::Middle => input.mouse_button_middle.set_state(is_pressed),
                        MouseButton::Right => input.mouse_button_right.set_state(is_pressed),
                        _ => {}
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    let is_pressed = false;
                    use sdl2::mouse::MouseButton;
                    match mouse_btn {
                        MouseButton::Left => input.mouse_button_left.set_state(is_pressed),
                        MouseButton::Middle => input.mouse_button_middle.set_state(is_pressed),
                        MouseButton::Right => input.mouse_button_right.set_state(is_pressed),
                        _ => {}
                    }
                }
                _ => (),
            }
        }

        if relative_mouse_mode_enabled && window_has_focus {
            mouse_pos_screen += mouse_delta_screen;
            mouse_pos_screen = mouse_pos_screen.clamped_in_rect(Rect::from_width_height(
                screen_dimensions.x - 1.0,
                screen_dimensions.y - 1.0,
            ));

            // TODO: Check if we need this in SDL
            //
            //             // TODO(JaSc): Maybe we need to set this more frequently?
            //             window
            //                 .set_cursor_position(glutin::dpi::LogicalPosition::new(
            //                     f64::from(screen_dimensions.x) / 2.0,
            //                     f64::from(screen_dimensions.y) / 2.0,
            //                 ))
            //                 .unwrap();
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

        let timer_audio = Timer::new();
        {
            // TODO: AUDIO
            //let mut audio_output_buffer = audio_output_buffer.lock().unwrap();
            //game_lib.process_audio(&input, &mut game_context, &mut audio_output_buffer);
        }
        input.time_audio = timer_audio.elapsed_time() as f32;

        // Process Systemcommands
        for command in game_context.get_system_commands() {
            match command {
                SystemCommand::EnableRelativeMouseMovementCapture(do_enable) => {
                    // TODO: Reimplement this
                    // window.hide_cursor(do_enable && window_has_focus);
                    relative_mouse_mode_enabled = do_enable;
                }
                SystemCommand::ShutdownGame => is_running = false,
            }
        }

        // Draw to screen
        let timer_draw = Timer::new();
        rc.process_draw_commands(game_context.get_draw_commands())
            .context("Could not to process a draw command")?;
        input.time_draw = timer_draw.elapsed_time() as f32;

        // Flush and flip buffers
        rc.encoder.flush(&mut device);

        window.gl_swap_window();
        device.cleanup();

        // Reset input
        input.prepare_for_next_frame();
    }

    Ok(())
}
