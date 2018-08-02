/*
TODO(JaSc):
  - Pixel perfect renderer with generalized (pixel independent) coordinate system
    x Render to offscreen buffer and blit to main screen
    - Static world camera 
    - Transformation mouse <-> screen <-> world 
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
    - Atlas textures and sprite/quad/line-batching
    - Atlas and font packer
    - Texture array of atlases implementation
    - Drawing debug overlays (grids/camera-frustums/crosshairs/depthbuffer)
    - Gamepad input
    - Mouse zooming
    - Raycasting and collision detection
    - Fixed sized and flexible sized pixel perfect canvases (framebuffers)
    - Live looped input playback and recording
    x Hot reloading of game code
*/

#[macro_use]
extern crate log;
extern crate fern;
extern crate image;
extern crate rand;

use libloading::Library;
use std::collections::HashMap;

extern crate game_lib;
use game_lib::{Color, Mat4, Point, Quad, Rect, SquareMatrix, Vertex, VertexIndex};

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
    unsafe { std::mem::transmute(vertices) }
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
    const CANVAS_WIDTH: u16 = 320;
    const CANVAS_HEIGHT: u16 = 180;
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
    let mut screen_rect = Rect::from_dimension(0.0, 0.0);
    let mut window_entered_fullscreen = false;

    let mut input = game_lib::GameInput::new();

    let mut game_lib = GameLib::new("target/debug/", "game_interface_glue");
    //
    info!("Entering main event loop");
    info!("------------------------");
    //
    while running {
        // Testing library hotreloading
        if game_lib.needs_reloading() {
            game_lib = game_lib.reload();
        }

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
                        info!("Window resized: {:?}", (new_dim.width, new_dim.height));
                        window.resize(new_dim.to_physical(window.get_hidpi_factor()));
                        gfx_window_glutin::update_views(
                            &window,
                            &mut rc.screen_pipeline_data.out_color,
                            &mut rc.screen_pipeline_data.out_depth,
                        );
                        screen_rect =
                            Rect::from_dimension(new_dim.width as f32, new_dim.height as f32);

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
                            (screen_rect.height - 1.0) - position.y as f32,
                        );
                    }
                    _ => (),
                }
            }
        });

        // Prepare input and update game
        // -----------------------------------------------------------------------------------------

        // NOTE: cursor_pos_canvas is in the following interval:
        //       [0 .. canvas_rect.width - 1] x [0 .. canvas_rect.height - 1]
        //       where (0,0) is the bottom left of the screen.
        let canvas_cursor_pos = rc.screen_coord_to_canvas_coord(screen_cursor_pos);
        let canvas_rect = rc.canvas_rect();
        input.canvas_width = canvas_rect.width as i32;
        input.canvas_height = canvas_rect.height as i32;
        input.cursor_pos_x = canvas_cursor_pos.x as i32;
        input.cursor_pos_y = canvas_cursor_pos.y as i32;

        let draw_commands = game_lib.update_and_draw(&input);

        // Draw into canvas
        // -----------------------------------------------------------------------------------------

        let clear_color = Color::new(0.7, 0.4, 0.2, 1.0);
        rc.clear_canvas(clear_color);

        for draw_command in draw_commands {
            rc.draw_into_canvas(
                draw_command.projection,
                &draw_command.texture,
                convert_to_gfx_format(&draw_command.vertices),
                &draw_command.indices,
            );
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
    canvas_pipeline_state_object: gfx::PipelineState<R, pipe::Meta>,

    textures: HashMap<String, gfx::handle::ShaderResourceView<R, [f32; 4]>>,
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
        info!("Creating screen and canvas pipelines");
        //
        let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
        let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();
        let screen_pipeline_state_object = factory
            .create_pipeline_simple(&vertex_shader, &fragment_shader, pipe::new())
            .expect("Failed to create a pipeline state object");
        let canvas_pipeline_state_object = factory
            .create_pipeline_simple(&vertex_shader, &fragment_shader, pipe::new())
            .expect("Failed to create a pipeline state object");

        //
        info!("Creating offscreen render targets");
        //
        let canvas_rect = Rect::from_dimension(f32::from(canvas_width), f32::from(canvas_heigth));
        let (_, canvas_shader_resource_view, canvas_color_render_target_view) = factory
            .create_render_target::<ColorFormat>(
                canvas_rect.width as u16,
                canvas_rect.height as u16,
            )
            .expect("Failed to create a canvas color render target");
        let canvas_depth_render_target_view = factory
            .create_depth_stencil_view_only::<DepthFormat>(
                canvas_rect.width as u16,
                canvas_rect.height as u16,
            )
            .expect("Failed to create a canvas depth render target");

        //
        info!("Creating dummy textures and sampler");
        //
        use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
        let sampler_info = SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile);
        let texture_sampler = factory.create_sampler(sampler_info);

        let mut textures = HashMap::new();
        textures.insert(
            "dummy".to_string(),
            debug_load_texture(&mut factory, "resources/dummy.png"),
        );
        textures.insert(
            "another_dummy".to_string(),
            debug_load_texture(&mut factory, "resources/another_dummy.png"),
        );

        //
        info!("Creating screen and canvas pipeline data");
        //
        let canvas_pipeline_data = pipe::Data {
            vertex_buffer: factory.create_vertex_buffer(&[]),
            texture: (textures["dummy"].clone(), texture_sampler.clone()),
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
            canvas_pipeline_state_object,
            textures,
        }
    }

    /// Returns the dimensions of the canvas in pixels as rectangle
    pub fn canvas_rect(&self) -> Rect {
        let (width, height, _, _) = self.canvas_pipeline_data.out_color.get_dimensions();
        Rect::from_dimension(f32::from(width), f32::from(height))
    }

    /// Returns the dimensions of the screen in pixels as rectangle
    pub fn screen_rect(&self) -> Rect {
        let (width, height, _, _) = self.screen_pipeline_data.out_color.get_dimensions();
        Rect::from_dimension(f32::from(width), f32::from(height))
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
        blit_rect.width -= 1.0;
        blit_rect.height -= 1.0;
        let blit_rect_point = game_lib::clamp_point_in_rect(screen_point, blit_rect);
        blit_rect.width += 1.0;
        blit_rect.height += 1.0;

        let canvas_rect = self.canvas_rect();
        Point::new(
            f32::floor(canvas_rect.width * ((blit_rect_point.x - blit_rect.x) / blit_rect.width)),
            f32::floor(canvas_rect.height * ((blit_rect_point.y - blit_rect.y) / blit_rect.height)),
        )
    }

    pub fn clear_canvas(&mut self, clear_color: Color) {
        self.encoder
            .clear(&self.canvas_pipeline_data.out_color, clear_color.into());
        self.encoder
            .clear_depth(&self.canvas_pipeline_data.out_depth, 1.0);
    }

    pub fn draw_into_canvas(
        &mut self,
        projection: Mat4,
        texture_name: &str,
        vertices: &[VertexGFX],
        indices: &[VertexIndex],
    ) {
        let (canvas_vertex_buffer, canvas_slice) = self
            .factory
            .create_vertex_buffer_with_slice(&vertices, &*indices);

        self.canvas_pipeline_data.texture.0 = self.textures[texture_name].clone();
        self.canvas_pipeline_data.vertex_buffer = canvas_vertex_buffer;
        self.canvas_pipeline_data.transform = projection.into();

        self.encoder.draw(
            &canvas_slice,
            &self.canvas_pipeline_state_object,
            &self.canvas_pipeline_data,
        );
    }

    pub fn blit_canvas_to_screen(&mut self, letterbox_color: Color) {
        let blit_quad = Quad::new(self.canvas_blit_rect(), 0.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let (vertices, indices) = blit_quad.into_vertices_indices(0);
        let (vertex_buffer, slice) = self
            .factory
            .create_vertex_buffer_with_slice(convert_to_gfx_format(&vertices), &indices[..]);
        self.screen_pipeline_data.vertex_buffer = vertex_buffer;

        // NOTE: The projection matrix is upside-down for correct rendering of the canvas
        let screen_rect = self.screen_rect();
        let projection_mat =
            game_lib::ortho(0.0, screen_rect.width, screen_rect.height, 0.0, -1.0, 1.0);
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
}

fn debug_load_texture<F, R>(
    factory: &mut F,
    file_name: &str,
) -> gfx::handle::ShaderResourceView<R, [f32; 4]>
where
    F: gfx::Factory<R>,
    R: gfx::Resources,
{
    use gfx::format::Rgba8;
    let img = image::open(file_name).unwrap().to_rgba();
    let (width, height) = img.dimensions();
    let kind = gfx::texture::Kind::D2(width as u16, height as u16, gfx::texture::AaMode::Single);
    let (_, view) = factory
        .create_texture_immutable_u8::<Rgba8>(kind, gfx::texture::Mipmap::Provided, &[&img])
        .unwrap();
    view
}

//==================================================================================================
// GameLib
//==================================================================================================
//
pub struct GameLib {
    pub lib: Library,
    lib_path: String,
    lib_name: String,
    last_modified_time: std::time::SystemTime,
    copy_counter: usize,
}

impl GameLib {
    pub fn update_and_draw(&self, input: &game_lib::GameInput) -> Vec<game_lib::DrawCommand> {
        unsafe {
            let f = self
                .lib
                .get::<fn(&game_lib::GameInput) -> Vec<game_lib::DrawCommand>>(b"update_and_draw\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not load `update_and_draw` function from GameLib: {}",
                        error
                    )
                });
            f(input)
        }
    }

    pub fn new(lib_path: &str, lib_name: &str) -> GameLib {
        GameLib::load(0, lib_path, lib_name)
    }

    pub fn needs_reloading(&mut self) -> bool {
        let (file_path, _, _) =
            GameLib::construct_paths(self.copy_counter, &self.lib_path, &self.lib_name);

        if let Ok(Ok(last_modified_time)) =
            std::fs::metadata(&file_path).map(|metadata| metadata.modified())
        {
            // NOTE: We do not set `self.last_modified_time` here because we might call this
            //       function multiple times and want the same result everytime until we reload
            last_modified_time > self.last_modified_time
        } else {
            false
        }
    }

    pub fn reload(self) -> GameLib {
        let lib_path = self.lib_path.clone();
        let lib_name = self.lib_name.clone();
        let mut copy_counter = self.copy_counter;

        if GameLib::copy_lib(copy_counter, &lib_path, &lib_name).is_err() {
            // NOTE: It can happen (even multiple times) that we fail to copy the library while
            //       it is being recompiled/updated. This is OK as we can just retry the next time.
            return self;
        }

        copy_counter += 1;
        drop(self);
        GameLib::load(copy_counter, &lib_path, &lib_name)
    }

    fn load(mut copy_counter: usize, lib_path: &str, lib_name: &str) -> GameLib {
        GameLib::copy_lib(copy_counter, lib_path, lib_name)
            .unwrap_or_else(|error| panic!("Error while copying: {}", error));
        let (file_path, _, copy_file_path) =
            GameLib::construct_paths(copy_counter, lib_path, lib_name);
        copy_counter += 1;

        // NOTE: Loading from a copy is necessary on MS Windows due to write protection issues
        let lib = Library::new(&copy_file_path).unwrap_or_else(|error| {
            panic!("Failed to load library {} : {}", copy_file_path, error)
        });

        let last_modified_time = std::fs::metadata(&file_path)
            .unwrap_or_else(|error| {
                panic!("Cannot open file {} to read metadata: {}", file_path, error)
            })
            .modified()
            .unwrap_or_else(|error| {
                panic!("Cannot read metadata of file {}: {}", file_path, error)
            });

        info!("Game lib reloaded");
        GameLib {
            lib,
            lib_path: String::from(lib_path),
            lib_name: String::from(lib_name),
            last_modified_time,
            copy_counter,
        }
    }

    /// Creates temp folder (if necessary) and copies our lib into it
    fn copy_lib(
        copy_counter: usize,
        lib_path: &str,
        lib_name: &str,
    ) -> Result<u64, std::io::Error> {
        // Construct necessary the file paths
        let (file_path, copy_path, copy_file_path) =
            GameLib::construct_paths(copy_counter, lib_path, lib_name);

        std::fs::create_dir_all(&copy_path)
            .unwrap_or_else(|error| panic!("Cannot create dir {}: {}", copy_path, error));

        // NOTE: Copy may fail while the library being rebuild
        let copy_result = std::fs::copy(&file_path, &copy_file_path);
        if let Err(ref error) = copy_result {
            warn!(
                "Cannot copy file {} to {}: {}",
                file_path, copy_file_path, error
            )
        }
        copy_result
    }

    fn construct_paths(
        copy_counter: usize,
        lib_path: &str,
        lib_name: &str,
    ) -> (String, String, String) {
        let file_path = String::from(lib_path) + &GameLib::lib_name_to_file_name(lib_name);
        let copy_path = String::from(lib_path) + "libcopies/";
        let copy_file_path = copy_path.clone()
            + &GameLib::lib_name_to_file_name(
                &(String::from(lib_name) + &copy_counter.to_string()),
            );

        (file_path, copy_path, copy_file_path)
    }

    #[cfg(target_os = "windows")]
    fn lib_name_to_file_name(lib_name: &str) -> String {
        format!("{}.dll", lib_name)
    }
    #[cfg(target_os = "linux")]
    fn lib_name_to_file_name(lib_name: &str) -> String {
        format!("lib{}.so", lib_name)
    }
}
