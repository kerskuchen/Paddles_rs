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
  - The following are things to remember to extract out of an old C project
    x Debug macro to print a variable and it's name quickly
    - Be able to conveniently do debug printing on screen
    - Moving camera system
    - Atlas textures and sprite/quad/line-batching
    - Atlas and font packer
    - Drawing debug overlays (grids/camera-frustums/crosshairs/depthbuffer)
    - Gamepad input
    - Mouse zooming
    - Raycasting and collision detection
    - Fixed sized and flexible sized pixel perfect canvases (framebuffers)
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

use gfx::traits::Factory;
use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::GlContext;

use cgmath::prelude::*;
use cgmath::Matrix4;
use rand::prelude::*;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 4] = "a_Pos",
        uv: [f32; 2] = "a_Uv",
        color: [f32; 4] = "a_Color",
    }

    pipeline screen_pipe {
        vertex_buffer: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4];4]> = "u_Transform",
        texture: gfx::TextureSampler<[f32; 4]> = "u_Sampler",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
    }

    pipeline canvas_pipe {
        vertex_buffer: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4];4]> = "u_Transform",
        texture: gfx::TextureSampler<[f32; 4]> = "u_Sampler",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

/// A macro used for debugging which returns a string containing the name and value of a given
/// variable.
///
/// It uses the `stringify` macro internally and requires the input to be an identifier.
///
/// # Examples
///
/// ```
/// let name = 5;
/// assert_eq!(dformat!(name), "name = 5");
/// ```
macro_rules! dformat {
    ($x:ident) => {
        format!("{} = {:?}", stringify!($x), $x)
    };
}

/// A macro used for debugging which prints a string containing the name and value of a given
/// variable.
///
/// It uses the `dformat` macro internally and requires the input to be an identifier.
/// For more information see the `dformat` macro
///
/// # Examples
///
/// ```
/// let name = 5;
/// dprintln!(name);
/// // prints: "name = 5"
/// ```
macro_rules! dprintln {
    ($x:ident) => {
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
    let fullscreen_monitor = match FULLSCREEN_MODE {
        true => Some(monitor),
        false => None,
    };
    let window_builder = glutin::WindowBuilder::new()
        .with_resizable(!FULLSCREEN_MODE)
        // TODO(JaSc): Allow cursor grabbing in windowed mode when 
        //             https://github.com/tomaka/winit/issues/574
        //             is fixed. Grabbing the cursor in windowed mode and ALT-TABBING in and out
        //             is currently broken.
        .with_fullscreen(fullscreen_monitor)
        .with_title("Pongi".to_string());
    let context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
        .with_vsync(true);
    let (window, mut device, mut factory, screen_rendertarget, mut screen_depth_rendertarget) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &events_loop);

    //
    info!("Creating command buffer and shaders");
    //
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
    let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();

    //
    info!("Creating dummy texture and sampler");
    //
    use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
    let sampler_info = SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile);
    let dummy_texture = debug_load_texture(&mut factory);
    let texture_sampler = factory.create_sampler(sampler_info);

    //
    info!("Creating offscreen render target and pipeline");
    //
    let (_, canvas_shader_resource_view, canvas_render_target_view) = factory
        .create_render_target::<ColorFormat>(320, 180)
        .unwrap();
    let canvas_depth_render_target_view = factory
        .create_depth_stencil_view_only::<DepthFormat>(320, 180)
        .unwrap();
    let canvas_pipeline_state_object = factory
        .create_pipeline_simple(&vertex_shader, &fragment_shader, canvas_pipe::new())
        .expect("Failed to create a pipeline state object");
    let mut canvas_pipeline_data = canvas_pipe::Data {
        vertex_buffer: factory.create_vertex_buffer(&[]),
        texture: (dummy_texture, texture_sampler.clone()),
        transform: Matrix4::identity().into(),
        out_color: canvas_render_target_view,
        out_depth: canvas_depth_render_target_view,
    };

    //
    info!("Creating screen pipeline");
    //
    let screen_pipeline_state_object = factory
        .create_pipeline_simple(&vertex_shader, &fragment_shader, screen_pipe::new())
        .expect("Failed to create a pipeline state object");
    let mut screen_pipeline_data = screen_pipe::Data {
        vertex_buffer: factory.create_vertex_buffer(&[]),
        texture: (canvas_shader_resource_view, texture_sampler),
        transform: Matrix4::identity().into(),
        out_color: screen_rendertarget,
    };

    //
    info!("Creating dummy scene");
    //
    let mut render_context = Rendercontext::new();
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let pos = (rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0), 0.5);
        let size = rng.gen_range(0.02, 0.4);
        let dim = (size, size);
        let color: [f32; 4] = [
            rng.gen_range(0.2, 0.9),
            rng.gen_range(0.2, 0.9),
            rng.gen_range(0.2, 0.9),
            1.0,
        ];
        render_context.add_quad(pos, dim, color);
    }
    //render_context.add_quad((0.0, 0.0), (1.0, 1.0), [1.0, 0.0, 0.0, 1.0]);

    // ---------------------------------------------------------------------------------------------
    // Main loop
    //

    // State variables
    let mut running = true;
    let mut cursor_pos = (0.0, 0.0);
    let mut window_dimensions: (f32, f32) = (0.0, 0.0);
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
                            &mut screen_pipeline_data.out_color,
                            &mut screen_depth_rendertarget,
                        );
                        window_dimensions = (new_dim.width as f32, new_dim.height as f32);

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
                        let cursor_x = position.x as f32 / window_dimensions.0;
                        let cursor_y = position.y as f32 / window_dimensions.1;
                        cursor_pos = (cursor_x - 0.5, -1.0 * cursor_y + 0.5);
                    }
                    _ => (),
                }
            }
        });

        // Aspect ratio correction for view and cursor
        let aspect_ratio = (window_dimensions.0 as f32) / (window_dimensions.1 as f32);
        let (width, height) = if aspect_ratio > 1.0 {
            (1.0 * aspect_ratio, 1.0)
        } else {
            (1.0, 1.0 / aspect_ratio)
        };
        let cursor_pos = if aspect_ratio > 1.0 {
            (cursor_pos.0 * aspect_ratio, cursor_pos.1)
        } else {
            (cursor_pos.0, cursor_pos.1 / aspect_ratio)
        };

        // Draw canvas
        // -----------------------------------------------------------------------------------------
        let projection_mat = cgmath::ortho(
            -0.5 * width,
            0.5 * width,
            -0.5 * height,
            0.5 * height,
            -1.0,
            1.0,
        );
        let (vertices, indices) = render_context.get_vertices_indices(cursor_pos);
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertices, &*indices);

        canvas_pipeline_data.transform = projection_mat.into();
        canvas_pipeline_data.vertex_buffer = vertex_buffer;

        const CANVAS_COLOR: [f32; 4] = [0.7, 0.4, 0.2, 1.0];
        encoder.clear(&canvas_pipeline_data.out_color, CANVAS_COLOR);
        encoder.clear_depth(&canvas_pipeline_data.out_depth, 1.0);
        encoder.draw(&slice, &canvas_pipeline_state_object, &canvas_pipeline_data);

        // Draw canvas to screen
        // -----------------------------------------------------------------------------------------
        const COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        const SQUARE: &[Vertex] = &[
            Vertex {
                pos: [1.0, -1.0, 0.0, 1.0],
                uv: [1.0, 0.0],
                color: COLOR,
            },
            Vertex {
                pos: [-1.0, -1.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: COLOR,
            },
            Vertex {
                pos: [-1.0, 1.0, 0.0, 1.0],
                uv: [0.0, 1.0],
                color: COLOR,
            },
            Vertex {
                pos: [1.0, 1.0, 0.0, 1.0],
                uv: [1.0, 1.0],
                color: COLOR,
            },
        ];
        const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(SQUARE, INDICES);

        screen_pipeline_data.vertex_buffer = vertex_buffer;

        encoder.clear(&screen_pipeline_data.out_color, COLOR);
        encoder.draw(&slice, &screen_pipeline_state_object, &screen_pipeline_data);

        encoder.flush(&mut device);
        window.swap_buffers().expect("Failed to swap framebuffers");
        device.cleanup();
    }
}

// fn draw_canvas_to_screen<C, R, F>(
//     factory: &mut F,
//     encoder: &mut gfx::Encoder<R, C>,
//     mut pipeline_data: pipe::Data<R>,
//     pipeline_state_object: &gfx::PipelineState<R, pipe::Meta>,
//     screen_rendertarget: gfx::handle::RenderTargetView<R, ColorFormat>,
//     screen_depth_rendertarget: gfx::handle::DepthStencilView<R, DepthFormat>,
//     canvas_shader_resource_view: gfx::handle::ShaderResourceView<R, [f32; 4]>,
// ) where
//     R: gfx::Resources,
//     C: gfx::CommandBuffer<R>,
//     F: gfx::Factory<R>,
// {
//     const COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
//     const SQUARE: &[Vertex] = &[
//         Vertex {
//             pos: [1.0, -1.0, 0.0, 1.0],
//             uv: [1.0, 0.0],
//             color: COLOR,
//         },
//         Vertex {
//             pos: [-1.0, -1.0, 0.0, 1.0],
//             uv: [0.0, 0.0],
//             color: COLOR,
//         },
//         Vertex {
//             pos: [-1.0, 1.0, 0.0, 1.0],
//             uv: [0.0, 1.0],
//             color: COLOR,
//         },
//         Vertex {
//             pos: [1.0, 1.0, 0.0, 1.0],
//             uv: [1.0, 1.0],
//             color: COLOR,
//         },
//     ];
//     const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];
//
//     let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(SQUARE, INDICES);
//     let projection_mat = Matrix4::identity().into();
//
//     pipeline_data.transform = projection_mat;
//     pipeline_data.vertex_buffer = vertex_buffer;
//     pipeline_data.texture.0 = canvas_shader_resource_view;
//     pipeline_data.out_color = screen_rendertarget;
//     pipeline_data.out_depth = screen_depth_rendertarget;
//
//     encoder.clear(&pipeline_data.out_color, COLOR);
//     encoder.clear_depth(&pipeline_data.out_depth, 1.0);
//     encoder.draw(&slice, &pipeline_state_object, &pipeline_data);
// }

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

#[derive(Debug)]
struct Rendercontext {
    quads: Vec<Quad>,
}

impl Rendercontext {
    fn new() -> Self {
        Rendercontext { quads: vec![] }
    }

    fn add_quad(&mut self, pos: (f32, f32, f32), dim: (f32, f32), color: [f32; 4]) {
        self.quads.push(Quad { pos, dim, color });
    }

    fn get_vertices_indices(&self, cursor_pos: (f32, f32)) -> (Vec<Vertex>, Vec<u16>) {
        let (mut vertices, mut indices) = (vec![], vec![]);

        // Fill vertices and indices arrays with quads
        for (quad_index, quad) in self.quads.iter().enumerate() {
            quad.append_vertices_indices((quad_index) as u16, &mut vertices, &mut indices);
        }

        // Add dummy quad for cursor
        const CURSOR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        let cursor_quad = Quad {
            pos: (cursor_pos.0, cursor_pos.1, 0.0),
            dim: (0.02, 0.02),
            color: CURSOR_COLOR,
        };
        cursor_quad.append_vertices_indices(self.quads.len() as u16, &mut vertices, &mut indices);

        (vertices, indices)
    }
}

#[derive(Debug, Clone, Copy)]
struct Quad {
    pos: (f32, f32, f32),
    dim: (f32, f32),
    color: [f32; 4],
}

impl Quad {
    fn append_vertices_indices(
        &self,
        quad_index: u16,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u16>,
    ) {
        let pos = self.pos;
        let half_width = 0.5 * self.dim.0;
        let half_height = 0.5 * self.dim.1;

        vertices.extend(&[
            Vertex {
                pos: [pos.0 - half_width, pos.1 - half_height, pos.2, 1.0],
                uv: [0.0, 1.0],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 + half_width, pos.1 - half_height, pos.2, 1.0],
                uv: [1.0, 1.0],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 + half_width, pos.1 + half_height, pos.2, 1.0],
                uv: [1.0, 0.0],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 - half_width, pos.1 + half_height, pos.2, 1.0],
                uv: [0.0, 0.0],
                color: self.color,
            },
        ]);

        indices.extend(&[
            4 * quad_index,
            4 * quad_index + 1,
            4 * quad_index + 2,
            4 * quad_index + 2,
            4 * quad_index + 3,
            4 * quad_index,
        ]);
    }
}
