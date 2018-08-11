/*
TODO(JaSc):
  X Pixel perfect renderer with generalized (pixel independent) coordinate system
    x Render to offscreen buffer and blit to main screen
    X Static world camera 
    X Transformation mouse <-> screen <-> world 
  - Atlas packer
  x Font packer
  - Atlas textures and sprite/quad/line-batching
  - Basic sprite loading and Bitmap font rendering (no sprite atlas yet)
  - Game input + keyboard/mouse-support
  - Gamestate + logic + timing
  - Audio playback
  - Some nice glowing shader effects
  - BG music with PHAT BEATSIES

BACKLOG(JaSc):
  - The following are things to remember to extract out of the old C project in the long term
    x Debug macro to print a variable and it's name quickly
    - Be able to conveniently do debug printing on screen
    - Moving camera system
    - Aseprite image parser and converter
    - Texture array of atlases implementation
    - Drawing debug overlays (grids/camera-frustums/crosshairs/depthbuffer)
    - Gamepad input
    x Mouse zooming
    - Raycasting and collision detection
    x Fixed sized pixel perfect canvase (framebuffer)
    - Flexible sized pixel perfect canvase (framebuffer)
    - Live looped input playback and recording
    x Hot reloading of game code
    - Disable hot reloading when making a publish build
*/

#[macro_use]
extern crate log;
extern crate fern;
extern crate rand;

extern crate game_lib;

use game_lib::{
    Color, ComponentBytes, DrawCommand, GameInput, Mat4, Mat4Helper, Pixel, Point, Quad, Rect,
    SquareMatrix, Texture, Vec2, Vertex, VertexIndex,
};

mod game_interface;
use game_interface::GameLib;
use std::collections::HashMap;

//==================================================================================================
// GFX-RS stuff
//==================================================================================================
//
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate libloading;

use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::GlContext;

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;

gfx_defines! {
    vertex VertexGFX {
        pos: [f32; 4] = "a_Pos",
        uv: [f32; 2] = "a_Uv",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vertex_buffer: gfx::VertexBuffer<VertexGFX> = (),
        transform: gfx::Global<[[f32; 4];4]> = "u_Transform",
        texture: gfx::TextureSampler<[f32; 4]> = "u_Sampler",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

/// Converts a slice of [`Vertex`] into a slice of [`VertexGFX`] for gfx to consume.
///
/// Note that both types are memory-equivalent so the conversion is just a transmutation
pub fn convert_to_gfx_format(vertices: &[Vertex]) -> &[VertexGFX] {
    unsafe { &*(vertices as *const [Vertex] as *const [VertexGFX]) }
}

//==================================================================================================
// Mainloop
//==================================================================================================
//
fn main() {
    // Initializing logger
    //
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}-{}: {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .level_for("gfx_device_gl", log::LevelFilter::Warn)
        .level_for("winit", log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()
        .expect("Could not initialize logger");

    // ---------------------------------------------------------------------------------------------
    // Video subsystem initialization
    //

    // TODO(JaSc): Read MONITOR_ID and FULLSCREEN_MODE from config file
    const MONITOR_ID: usize = 0;
    const FULLSCREEN_MODE: bool = true;
    const CANVAS_WIDTH: u16 = 480;
    const CANVAS_HEIGHT: u16 = 270;
    const GL_VERSION_MAJOR: u8 = 3;
    const GL_VERSION_MINOR: u8 = 2;

    //
    info!("Getting monitor and its properties");
    //
    let mut events_loop = glutin::EventsLoop::new();
    let monitor = events_loop
        .get_available_monitors()
        .nth(MONITOR_ID)
        .unwrap_or_else(|| {
            panic!("No monitor with id {} found", MONITOR_ID);
        });

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
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
    );

    // ---------------------------------------------------------------------------------------------
    // Main loop
    //

    // State variables
    let mut running = true;
    let mut screen_cursor_pos = Point::new(0.0, 0.0);
    let mut screen_rect = Rect::zero();
    let mut window_entered_fullscreen = false;

    let mut input = GameInput::new();
    let mut game_lib = GameLib::new("target/debug/", "game_interface_glue");

    let mut game_state = game_lib.initialize(i32::from(CANVAS_WIDTH), i32::from(CANVAS_HEIGHT));

    //
    info!("Entering main event loop");
    info!("------------------------");
    //
    while running {
        // Testing library hotreloading
        if game_lib.needs_reloading() {
            game_lib = game_lib.reload();
        }

        input.clear_button_transitions();

        use glutin::{Event, KeyboardInput, WindowEvent};
        events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => running = false,
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
                            Escape => running = false,
                            _ => (),
                        }
                    }
                    WindowEvent::Focused(has_focus) => {
                        info!("Window has focus: {}", has_focus);
                        if FULLSCREEN_MODE && window_entered_fullscreen {
                            // NOTE: We need to grab/ungrab mouse cursor when ALT-TABBING in and out
                            //       or the user cannot use their computer correctly in a
                            //       multi-monitor setup while running our app.
                            window.grab_cursor(has_focus).unwrap();
                        }
                    }
                    WindowEvent::Resized(new_dim) => {
                        window.resize(new_dim.to_physical(window.get_hidpi_factor()));
                        gfx_window_glutin::update_views(
                            &window,
                            &mut rc.screen_pipeline_data.out_color,
                            &mut rc.screen_pipeline_data.out_depth,
                        );
                        screen_rect =
                            Rect::from_width_height(new_dim.width as f32, new_dim.height as f32);

                        info!("=====================");
                        info!(
                            "Window resized: {} x {}",
                            rc.screen_rect().width(),
                            rc.screen_rect().height()
                        );
                        info!(
                            "Canvas size: {} x {}",
                            rc.canvas_rect().width(),
                            rc.canvas_rect().height()
                        );
                        info!("Blit-rect: {:?}", rc.canvas_blit_rect());
                        info!(
                            "Pixel scale factor: {} ",
                            if rc.canvas_blit_rect().pos.x == 0.0 {
                                rc.screen_rect().width() / rc.canvas_rect().width()
                            } else {
                                rc.screen_rect().height() / rc.canvas_rect().height()
                            }
                        );
                        info!(
                            "Pixel waste: {} x {}",
                            rc.screen_rect().width() - rc.canvas_blit_rect().width(),
                            rc.screen_rect().height() - rc.canvas_blit_rect().height(),
                        );
                        info!("=====================");

                        // Grab mouse cursor in window
                        // NOTE: Due to https://github.com/tomaka/winit/issues/574 we need to first
                        //       make sure that our resized window now spans the full screen before
                        //       we allow grabbing the mouse cursor.
                        // TODO(JaSc): Remove workaround when upstream is fixed
                        if FULLSCREEN_MODE && new_dim == monitor_logical_dimensions {
                            // Our window now has its final size, we can safely grab the cursor now
                            info!("Mouse cursor grabbed");
                            window_entered_fullscreen = true;
                            window.grab_cursor(true).unwrap();
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        // NOTE: cursor_pos_screen is in the following interval:
                        //       [0 .. screen_rect.width - 1] x [0 .. screen_rect.height - 1]
                        //       where (0,0) is the bottom left of the screen
                        screen_cursor_pos = Point::new(
                            position.x as f32,
                            (screen_rect.height() - 1.0) - position.y as f32,
                        );
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

        // Prepare input and update game
        // -----------------------------------------------------------------------------------------

        // NOTE: We add (0.5, 0,5) to the cursors' pixel-position as we want the cursor to be in the
        //       center of the canvas' pixel. This prevents artifacts when pixel-snapping the
        //       cursor world-position later.
        // Example:
        // If we transform canvas cursor pixel-position (2,0) to its world position and back to its
        // canvas pixel-position we get (1.9999981, 0.0). If we would pixel-snap this coordinate
        // (effectively flooring it), we would get (1.0, 0.0) which would be wrong.
        // Adding 0.5 gives us a correct flooring result.
        //
        let canvas_cursor_pos =
            rc.screen_coord_to_canvas_coord(screen_cursor_pos) + Vec2::new(0.5, 0.5);
        let canvas_cursor_pos_relative = canvas_cursor_pos / rc.canvas_rect().dim;
        input.mouse_pos_screen = canvas_cursor_pos_relative;

        let draw_commands = game_lib.update_and_draw(&input, &mut game_state);

        // Draw into canvas
        // -----------------------------------------------------------------------------------------

        let clear_color = Color::new(0.7, 0.4, 0.2, 1.0);
        rc.clear_canvas(clear_color);

        for draw_command in draw_commands {
            match draw_command {
                DrawCommand::UploadTexture { texture, pixels } => rc.add_texture(texture, pixels),
                DrawCommand::DrawFilled {
                    transform,
                    vertices,
                    indices,
                    texture,
                } => rc.draw_into_canvas_filled(
                    transform,
                    texture,
                    convert_to_gfx_format(&vertices),
                    &indices,
                ),
                DrawCommand::DrawLines {
                    transform,
                    vertices,
                    indices,
                    texture,
                } => rc.draw_into_canvas_lines(
                    transform,
                    texture,
                    convert_to_gfx_format(&vertices),
                    &indices,
                ),
            }
        }

        // Draw to screen and flip
        // -----------------------------------------------------------------------------------------
        let letterbox_color = Color::new(1.0, 0.4, 0.7, 1.0);
        rc.blit_canvas_to_screen(letterbox_color);
        rc.encoder.flush(&mut device);
        window.swap_buffers().expect("Failed to swap framebuffers");
        device.cleanup();
    }
}

//==================================================================================================
// RenderingContext
//==================================================================================================
//
pub struct RenderingContext<C, R, F>
where
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    F: gfx::Factory<R>,
{
    factory: F,
    encoder: gfx::Encoder<R, C>,

    screen_pipeline_data: pipe::Data<R>,
    screen_pipeline_state_object: gfx::PipelineState<R, pipe::Meta>,

    canvas_pipeline_data: pipe::Data<R>,
    canvas_pipeline_state_object_fill: gfx::PipelineState<R, pipe::Meta>,
    canvas_pipeline_state_object_line: gfx::PipelineState<R, pipe::Meta>,

    textures: HashMap<Texture, gfx::handle::ShaderResourceView<R, [f32; 4]>>,
}

impl<C, R, F> RenderingContext<C, R, F>
where
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    F: gfx::Factory<R>,
{
    pub fn new(
        mut factory: F,
        encoder: gfx::Encoder<R, C>,
        screen_color_render_target_view: gfx::handle::RenderTargetView<R, ColorFormat>,
        screen_depth_render_target_view: gfx::handle::DepthStencilView<R, DepthFormat>,
        canvas_width: u16,
        canvas_heigth: u16,
    ) -> RenderingContext<C, R, F> {
        //
        info!("Creating shader set");
        //
        let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
        let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();
        let shader_set = factory
            .create_shader_set(&vertex_shader, &fragment_shader)
            .unwrap_or_else(|error| {
                panic!("Could not create shader set: {}", error);
            });

        //
        info!("Creating screen pipeline state object");
        //
        let screen_pipeline_state_object = factory
            .create_pipeline_simple(&vertex_shader, &fragment_shader, pipe::new())
            .unwrap_or_else(|error| {
                panic!("Failed to create screen-pipeline state object: {}", error);
            });

        //
        info!("Creating canvas pipeline state object for line drawing");
        //
        use gfx::state::{CullFace, FrontFace, RasterMethod, Rasterizer};
        let line_rasterizer = Rasterizer {
            front_face: FrontFace::CounterClockwise,
            cull_face: CullFace::Nothing,
            method: RasterMethod::Line(1),
            offset: None,
            samples: None,
        };
        let canvas_pipeline_state_object_line = factory
            .create_pipeline_state(
                &shader_set,
                gfx::Primitive::LineList,
                line_rasterizer,
                pipe::new(),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "Failed to create canvas pipeline state object for line drawing: {}",
                    error
                );
            });

        //
        info!("Creating canvas pipeline state object for filled polygon drawing");
        //
        let fill_rasterizer = Rasterizer {
            front_face: FrontFace::CounterClockwise,
            cull_face: CullFace::Nothing,
            method: RasterMethod::Fill,
            offset: None,
            samples: None,
        };
        let canvas_pipeline_state_object_fill = factory
            .create_pipeline_state(
                &shader_set,
                gfx::Primitive::TriangleList,
                fill_rasterizer,
                pipe::new(),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "Failed to create canvas pipeline state object for fill drawing: {}",
                    error
                );
            });

        //
        info!("Creating offscreen render targets");
        //
        let canvas_rect =
            Rect::from_width_height(f32::from(canvas_width), f32::from(canvas_heigth));
        let (_, canvas_shader_resource_view, canvas_color_render_target_view) = factory
            .create_render_target::<ColorFormat>(
                canvas_rect.width() as u16,
                canvas_rect.height() as u16,
            )
            .expect("Failed to create a canvas color render target");
        let canvas_depth_render_target_view = factory
            .create_depth_stencil_view_only::<DepthFormat>(
                canvas_rect.width() as u16,
                canvas_rect.height() as u16,
            )
            .expect("Failed to create a canvas depth render target");

        //
        info!("Creating empty default texture and sampler");
        //
        use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
        let sampler_info = SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile);
        let texture_sampler = factory.create_sampler(sampler_info);

        // TODO(JaSc): Clean this up
        use gfx::format::Rgba8;
        let pixels = vec![0, 0, 0, 0];
        let kind = gfx::texture::Kind::D2(1, 1, gfx::texture::AaMode::Single);
        let (_, empty_texture) = factory
            .create_texture_immutable_u8::<Rgba8>(kind, gfx::texture::Mipmap::Provided, &[&pixels])
            .unwrap();

        //
        info!("Creating screen and canvas pipeline data");
        //
        let canvas_pipeline_data = pipe::Data {
            vertex_buffer: factory.create_vertex_buffer(&[]),
            texture: (empty_texture, texture_sampler.clone()),
            transform: Mat4::identity().into(),
            out_color: canvas_color_render_target_view,
            out_depth: canvas_depth_render_target_view,
        };
        let screen_pipeline_data = pipe::Data {
            vertex_buffer: factory.create_vertex_buffer(&[]),
            texture: (canvas_shader_resource_view, texture_sampler),
            transform: Mat4::identity().into(),
            out_color: screen_color_render_target_view,
            out_depth: screen_depth_render_target_view,
        };

        RenderingContext {
            factory,
            encoder,
            screen_pipeline_data,
            canvas_pipeline_data,
            screen_pipeline_state_object,
            canvas_pipeline_state_object_line,
            canvas_pipeline_state_object_fill,
            textures: HashMap::new(),
        }
    }

    /// Returns the dimensions of the canvas in pixels as rectangle
    pub fn canvas_rect(&self) -> Rect {
        let (width, height, _, _) = self.canvas_pipeline_data.out_color.get_dimensions();
        Rect::from_width_height(f32::from(width), f32::from(height))
    }

    /// Returns the dimensions of the screen in pixels as rectangle
    pub fn screen_rect(&self) -> Rect {
        let (width, height, _, _) = self.screen_pipeline_data.out_color.get_dimensions();
        Rect::from_width_height(f32::from(width), f32::from(height))
    }

    /// Returns the dimensions of the `blit_rectangle` of the canvas in pixels.
    /// The `blit-rectange` is the area of the screen where the content of the canvas is drawn onto.
    /// It is as big as a canvas that is proportionally stretched and centered to fill the whole
    /// screen.
    ///
    /// It may or may not be smaller than the full screen size depending on the aspect
    /// ratio of both the screen and the canvas. The `blit_rectange` is guaranteed to either have
    /// the same width a as the screen (with letterboxing if needed) or the same height as the
    /// screen (with columnboxing if needed) or completely fill the screen.
    pub fn canvas_blit_rect(&self) -> Rect {
        let screen_rect = self.screen_rect();
        self.canvas_rect()
            .stretched_to_fit(screen_rect)
            .centered_in(screen_rect)
    }

    /// Clamps a given `screen_point` to the area of the
    /// [`canvas_blit_rect`](#method.canvas_blit_rect) and converts the result into
    /// a canvas-position in the following interval:
    /// `[0..canvas_rect.width-1]x[0..canvas_rect.height-1]`
    /// where `(0,0)` is the bottom left of the canvas.
    pub fn screen_coord_to_canvas_coord(&self, screen_point: Point) -> Point {
        // NOTE: Clamping the point needs to use integer arithmetic such that
        //          x != canvas.rect.width and y != canvas.rect.height
        //       holds. We therefore need to subtract one from the blit_rect's dimension and then
        //       add one again after clamping to achieve the desired effect.
        // TODO(JaSc): Maybe make this more self documenting via integer rectangles
        let mut blit_rect = self.canvas_blit_rect();
        blit_rect.dim -= 1.0;
        let clamped_point = screen_point.clamped_in_rect(blit_rect);
        blit_rect.dim += 1.0;

        let result = self.canvas_rect().dim * ((clamped_point - blit_rect.pos) / blit_rect.dim);
        Point::new(f32::floor(result.x), f32::floor(result.y))
    }

    pub fn clear_canvas(&mut self, clear_color: Color) {
        self.encoder
            .clear(&self.canvas_pipeline_data.out_color, clear_color.into());
        self.encoder
            .clear_depth(&self.canvas_pipeline_data.out_depth, 1.0);
    }

    // TODO(JaSc): Remove duplicates
    pub fn draw_into_canvas_filled(
        &mut self,
        projection: Mat4,
        texture: Texture,
        vertices: &[VertexGFX],
        indices: &[VertexIndex],
    ) {
        let (canvas_vertex_buffer, canvas_slice) = self
            .factory
            .create_vertex_buffer_with_slice(&vertices, &*indices);

        self.canvas_pipeline_data.texture.0 = self
            .textures
            .get(&texture)
            .expect(&format!("Could not find texture {:?}", texture))
            .clone();

        self.canvas_pipeline_data.vertex_buffer = canvas_vertex_buffer;
        self.canvas_pipeline_data.transform = projection.into();

        self.encoder.draw(
            &canvas_slice,
            &self.canvas_pipeline_state_object_fill,
            &self.canvas_pipeline_data,
        );
    }

    pub fn draw_into_canvas_lines(
        &mut self,
        projection: Mat4,
        texture: Texture,
        vertices: &[VertexGFX],
        indices: &[VertexIndex],
    ) {
        let (canvas_vertex_buffer, canvas_slice) = self
            .factory
            .create_vertex_buffer_with_slice(&vertices, &*indices);

        self.canvas_pipeline_data.texture.0 = self
            .textures
            .get(&texture)
            .expect(&format!("Could not find texture {:?}", texture))
            .clone();
        self.canvas_pipeline_data.vertex_buffer = canvas_vertex_buffer;
        self.canvas_pipeline_data.transform = projection.into();

        self.encoder.draw(
            &canvas_slice,
            &self.canvas_pipeline_state_object_line,
            &self.canvas_pipeline_data,
        );
    }

    pub fn blit_canvas_to_screen(&mut self, letterbox_color: Color) {
        let blit_quad = Quad::new(self.canvas_blit_rect(), 0.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let vertices = blit_quad.into_vertices();
        let indices: [VertexIndex; 6] = [0, 1, 2, 2, 3, 0];
        let (vertex_buffer, slice) = self
            .factory
            .create_vertex_buffer_with_slice(convert_to_gfx_format(&vertices), &indices[..]);
        self.screen_pipeline_data.vertex_buffer = vertex_buffer;

        // NOTE: The projection matrix is flipped upside-down for correct rendering of the canvas
        let screen_rect = self.screen_rect();
        let projection_mat =
            Mat4::ortho_bottom_left_flipped_y(screen_rect.width(), screen_rect.height(), 0.0, 1.0);
        self.screen_pipeline_data.transform = projection_mat.into();

        self.encoder
            .clear(&self.screen_pipeline_data.out_color, letterbox_color.into());
        self.encoder
            .clear_depth(&self.screen_pipeline_data.out_depth, 1.0);
        self.encoder.draw(
            &slice,
            &self.screen_pipeline_state_object,
            &self.screen_pipeline_data,
        );
    }

    fn add_texture(&mut self, texture: Texture, pixels: Vec<Pixel>) {
        use gfx::format::Rgba8;
        let kind =
            gfx::texture::Kind::D2(texture.width, texture.height, gfx::texture::AaMode::Single);
        let (_, view) = self
            .factory
            .create_texture_immutable_u8::<Rgba8>(
                kind,
                gfx::texture::Mipmap::Provided,
                &[(&pixels).as_bytes()],
            )
            .unwrap();

        self.textures.insert(texture, view);
    }
}
