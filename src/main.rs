/*
TODO(JaSc):
  - Pixel perfect renderer with generalized (pixel independent) coordinate system
    - Render to offscreen buffer and blit to main screen
    - Static world camera 
    - Transformation mouse <-> screen <-> world 
  - Basic sprite loading and Bitmap font rendering (no sprite atlas yet)
  - Game input + keyboard/mouse-support
  - Gamestate + logic + timing
  - Audio playback
  - Some nice glowing shader effects
  - BG music with PHAT BEATSIES

BACKLOG(JaSc):
  - The following are things to remember to extract out of an old C project in the long term
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
*/

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;

#[macro_use]
extern crate log;
extern crate cgmath;
extern crate fern;
extern crate image;
extern crate rand;

use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::GlContext;

use cgmath::prelude::*;

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;
type Point = cgmath::Point2<f32>;
type Vec2 = cgmath::Vector2<f32>;
type Color = cgmath::Vector4<f32>;
type Mat4 = cgmath::Matrix4<f32>;
type VertexIndex = u16;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 4] = "a_Pos",
        uv: [f32; 2] = "a_Uv",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vertex_buffer: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4];4]> = "u_Transform",
        texture: gfx::TextureSampler<[f32; 4]> = "u_Sampler",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

fn clamp(val: f32, min: f32, max: f32) -> f32 {
    f32::max(min, f32::min(val, max))
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_dimension(width: f32, height: f32) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    pub fn from_corners(bottom_left: Point, top_right: Point) -> Rect {
        Rect {
            x: bottom_left.x,
            y: bottom_left.y,
            width: top_right.x - bottom_left.x,
            height: top_right.y - bottom_left.y,
        }
    }

    pub fn unit_rect_centered() -> Rect {
        Rect {
            x: -0.5,
            y: -0.5,
            width: 1.0,
            height: 1.0,
        }
    }

    /// Returns the biggest proportionally stretched version of the rectangle that can fit
    /// into `target`.
    pub fn stretched_to_fit(self, target: Rect) -> Rect {
        let source_aspect_ratio = self.width / self.height;
        let target_aspect_ratio = target.width / target.height;

        let scale_factor = if source_aspect_ratio < target_aspect_ratio {
            // Target rect is 'wider' than ours -> height is our limit when stretching
            target.height / self.height
        } else {
            // Target rect is 'narrower' than ours -> width is our limit when stretching
            target.width / self.width
        };

        let stretched_width = self.width * scale_factor;
        let stretched_height = self.height * scale_factor;

        Rect {
            x: self.x,
            y: self.x,
            width: stretched_width,
            height: stretched_height,
        }
    }

    /// Returns a version of the rectangle that is centered in `target`.
    pub fn centered_in(self, target: Rect) -> Rect {
        let x_offset_centered = target.x + (target.width - self.width) / 2.0;
        let y_offset_centered = target.y + (target.height - self.height) / 2.0;

        Rect {
            x: x_offset_centered,
            y: y_offset_centered,
            width: self.width,
            height: self.height,
        }
    }
    pub fn to_pos(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn to_dim(&self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }
}

fn clamp_point_in_rect(point: Point, rect: Rect) -> Point {
    Point {
        x: clamp(point.x, rect.x, rect.x + rect.width),
        y: clamp(point.y, rect.y, rect.y + rect.height),
    }
}

/// A macro for debugging which returns a string representation of an expression and its value
///
/// It uses the `stringify` macro internally and requires the input to be an expression.
///
/// # Examples
///
/// ```
/// let name = 5;
/// assert_eq!(dformat!(1 + 2), "1 + 2 = 3");
/// assert_eq!(dformat!(1 + name), "1 + name = 6");
/// assert_eq!(dformat!(name), "name = 5");
/// ```
#[allow(unused_macros)]
macro_rules! dformat {
    ($x:expr) => {
        format!("{} = {:?}", stringify!($x), $x)
    };
}

/// A macro used for debugging which prints a string containing the name and value of a given
/// variable.
///
/// It uses the `dformat` macro internally and requires the input to be an expression.
/// For more information see the `dformat` macro
///
/// # Examples
///
/// ```
/// dprintln!(1 + 2);
/// // prints: "1 + 2 = 3"
///
/// let name = 5;
/// dprintln!(name);
/// // prints: "name = 5"
///
/// dprintln!(1 + name);
/// // prints: "1 + name = 6"
/// ```
#[allow(unused_macros)]
macro_rules! dprintln {
    ($x:expr) => {
        println!("{}", dformat!($x));
    };
}

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
    const FULLSCREEN_MODE: bool = false;
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
    let mut cursor_pos_screen = Point::new(0.0, 0.0);

    let mut screen_rect = Rect::from_dimension(0.0, 0.0);
    let mut window_entered_fullscreen = false;

    //
    info!("Entering main event loop");
    info!("------------------------");
    //
    while running {
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
                        let cursor_x = position.x as f32;
                        let cursor_y = position.y as f32;
                        cursor_pos_screen = Point::new(cursor_x, screen_rect.height - cursor_y);
                    }
                    _ => (),
                }
            }
        });

        // Cursor position
        // -----------------------------------------------------------------------------------------

        let canvas_rect = rc.canvas_rect();
        let blit_rect = canvas_rect
            .stretched_to_fit(screen_rect)
            .centered_in(screen_rect);

        let cursor_pos_canvas = clamp_point_in_rect(cursor_pos_screen, blit_rect);
        let cursor_pos_canvas = Point::new(
            canvas_rect.width * ((cursor_pos_canvas.x - blit_rect.x) / blit_rect.width - 0.5),
            canvas_rect.height * ((cursor_pos_canvas.y - blit_rect.y) / blit_rect.height - 0.5),
        );

        // Draw into canvas
        // -----------------------------------------------------------------------------------------
        let projection_mat = cgmath::ortho(
            -0.5 * canvas_rect.width,
            0.5 * canvas_rect.width,
            -0.5 * canvas_rect.height,
            0.5 * canvas_rect.height,
            -1.0,
            1.0,
        );

        let quad_color = Color::new(1.0, 0.0, 0.0, 1.0);
        let cursor_color = Color::new(0.0, 0.0, 0.0, 1.0);

        // Add dummy quad for cursor
        let dummy_quad = Quad::new(
            Rect::from_dimension(canvas_rect.height, canvas_rect.height),
            -0.7,
            quad_color,
        );
        let cursor_quad = Quad::new(
            Rect::new(cursor_pos_canvas.x, cursor_pos_canvas.y, 16.0, 16.0),
            -0.5,
            cursor_color,
        );

        let (mut vertices, mut indices) = (vec![], vec![]);
        dummy_quad.append_vertices_indices_centered(0, &mut vertices, &mut indices);
        cursor_quad.append_vertices_indices_centered(1, &mut vertices, &mut indices);

        let clear_color = Color::new(0.7, 0.4, 0.2, 1.0);
        rc.clear_canvas(clear_color);
        rc.draw_to_canvas(projection_mat, &vertices, &indices);

        // Draw to screen and flip
        // -----------------------------------------------------------------------------------------
        let letterbox_color = Color::new(1.0, 0.4, 0.7, 1.0);
        rc.blit_canvas_to_screen(letterbox_color);
        rc.encoder.flush(&mut device);
        window.swap_buffers().expect("Failed to swap framebuffers");
        device.cleanup();
    }
}

struct RenderingContext<C, R, F>
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
}

impl<C, R, F> RenderingContext<C, R, F>
where
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    F: gfx::Factory<R>,
{
    fn new(
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
        info!("Creating dummy texture and sampler");
        //
        use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
        let sampler_info = SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile);
        let dummy_texture = debug_load_texture(&mut factory);
        let texture_sampler = factory.create_sampler(sampler_info);

        //
        info!("Creating screen and canvas pipeline data");
        //
        let canvas_pipeline_data = pipe::Data {
            vertex_buffer: factory.create_vertex_buffer(&[]),
            texture: (dummy_texture, texture_sampler.clone()),
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
        }
    }

    fn canvas_rect(&self) -> Rect {
        let (width, height, _, _) = self.canvas_pipeline_data.out_color.get_dimensions();
        Rect::from_dimension(f32::from(width), f32::from(height))
    }

    fn screen_rect(&self) -> Rect {
        let (width, height, _, _) = self.screen_pipeline_data.out_color.get_dimensions();
        Rect::from_dimension(f32::from(width), f32::from(height))
    }

    fn canvas_blit_rect(&self) -> Rect {
        let screen_rect = self.screen_rect();
        self.canvas_rect()
            .stretched_to_fit(screen_rect)
            .centered_in(screen_rect)
    }

    fn clear_canvas(&mut self, clear_color: Color) {
        self.encoder
            .clear(&self.canvas_pipeline_data.out_color, clear_color.into());
        self.encoder
            .clear_depth(&self.canvas_pipeline_data.out_depth, 1.0);
    }

    fn draw_to_canvas(&mut self, projection: Mat4, vertices: &[Vertex], indices: &[VertexIndex]) {
        let (canvas_vertex_buffer, canvas_slice) = self
            .factory
            .create_vertex_buffer_with_slice(&vertices, &*indices);

        self.canvas_pipeline_data.vertex_buffer = canvas_vertex_buffer;
        self.canvas_pipeline_data.transform = projection.into();

        self.encoder.draw(
            &canvas_slice,
            &self.canvas_pipeline_state_object,
            &self.canvas_pipeline_data,
        );
    }

    fn blit_canvas_to_screen(&mut self, letterbox_color: Color) {
        let quad = Quad::new(self.canvas_blit_rect(), 0.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let (vertices, indices) = quad.into_vertices_indices(0);
        let (vertex_buffer, slice) = self
            .factory
            .create_vertex_buffer_with_slice(&vertices, &indices[..]);
        self.screen_pipeline_data.vertex_buffer = vertex_buffer;

        // NOTE: The projection matrix is upside-down for correct rendering of the canvas
        let screen_rect = self.screen_rect();
        let projection_mat =
            cgmath::ortho(0.0, screen_rect.width, screen_rect.height, 0.0, -1.0, 1.0);
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

fn debug_load_texture<F, R>(factory: &mut F) -> gfx::handle::ShaderResourceView<R, [f32; 4]>
where
    F: gfx::Factory<R>,
    R: gfx::Resources,
{
    use gfx::format::Rgba8;
    let img = image::open("resources/dummy.png").unwrap().to_rgba();
    let (width, height) = img.dimensions();
    let kind = gfx::texture::Kind::D2(width as u16, height as u16, gfx::texture::AaMode::Single);
    let (_, view) = factory
        .create_texture_immutable_u8::<Rgba8>(kind, gfx::texture::Mipmap::Provided, &[&img])
        .unwrap();
    view
}

#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub rect: Rect,
    pub depth: f32,
    pub color: Color,
}

impl Quad {
    pub fn new(rect: Rect, depth: f32, color: Color) -> Quad {
        Quad { rect, depth, color }
    }

    pub fn unit_quad(depth: f32, color: Color) -> Quad {
        Quad {
            rect: Rect::from_dimension(1.0, 1.0),
            depth,
            color,
        }
    }

    // TODO(JaSc): Create vertex/index-buffer struct and move the `append_..` methods into that
    pub fn append_vertices_indices(
        &self,
        quad_index: VertexIndex,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<VertexIndex>,
    ) {
        let (self_vertices, self_indices) = self.into_vertices_indices(quad_index);
        vertices.extend(&self_vertices);
        indices.extend(&self_indices);
    }

    pub fn append_vertices_indices_centered(
        &self,
        quad_index: VertexIndex,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<VertexIndex>,
    ) {
        let (self_vertices, self_indices) = self.into_vertices_indices_centered(quad_index);
        vertices.extend(&self_vertices);
        indices.extend(&self_indices);
    }

    pub fn into_vertices_indices(self, quad_index: VertexIndex) -> ([Vertex; 4], [VertexIndex; 6]) {
        let pos = self.rect.to_pos();
        let dim = self.rect.to_dim();
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let vertices: [Vertex; 4] = [
            Vertex {
                pos: [pos.x, pos.y, depth, 1.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + dim.x, pos.y, depth, 1.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + dim.x, pos.y + dim.y, depth, 1.0],
                uv: [1.0, 0.0],
                color,
            },
            Vertex {
                pos: [pos.x, pos.y + dim.y, depth, 1.0],
                uv: [0.0, 0.0],
                color,
            },
        ];

        let indices: [VertexIndex; 6] = [
            4 * quad_index,
            4 * quad_index + 1,
            4 * quad_index + 2,
            4 * quad_index + 2,
            4 * quad_index + 3,
            4 * quad_index,
        ];

        (vertices, indices)
    }

    pub fn into_vertices_indices_centered(
        self,
        quad_index: VertexIndex,
    ) -> ([Vertex; 4], [VertexIndex; 6]) {
        let pos = self.rect.to_pos();
        let half_dim = 0.5 * self.rect.to_dim();
        let color = self.color.into();
        let depth = self.depth;

        // NOTE: UVs y-axis is intentionally flipped to prevent upside-down images
        let vertices: [Vertex; 4] = [
            Vertex {
                pos: [pos.x - half_dim.x, pos.y - half_dim.y, depth, 1.0],
                uv: [0.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + half_dim.x, pos.y - half_dim.y, depth, 1.0],
                uv: [1.0, 1.0],
                color,
            },
            Vertex {
                pos: [pos.x + half_dim.x, pos.y + half_dim.y, depth, 1.0],
                uv: [1.0, 0.0],
                color,
            },
            Vertex {
                pos: [pos.x - half_dim.x, pos.y + half_dim.y, depth, 1.0],
                uv: [0.0, 0.0],
                color,
            },
        ];

        let indices: [VertexIndex; 6] = [
            4 * quad_index,
            4 * quad_index + 1,
            4 * quad_index + 2,
            4 * quad_index + 2,
            4 * quad_index + 3,
            4 * quad_index,
        ];

        (vertices, indices)
    }
}
