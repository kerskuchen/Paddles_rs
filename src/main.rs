#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;

#[macro_use]
extern crate log;
extern crate fern;
extern crate image;
extern crate rand;

use gfx::traits::Factory;
use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::GlContext;
use rand::prelude::*;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        uv: [f32; 2] = "a_Uv",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vertex_buffer: gfx::VertexBuffer<Vertex> = (),
        texture: gfx::TextureSampler<[f32; 4]> = "u_sampler",
        target: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

fn main() {
    //
    info!("Initializing logger");
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

    //
    info!("Getting monitor and its properties");
    //
    let mut events_loop = glutin::EventsLoop::new();
    // TODO(JaSc): Make config to choose on which monitor to start the app
    let monitor_id = 0;
    let monitor = events_loop
        .get_available_monitors()
        .nth(monitor_id)
        .unwrap_or_else(|| {
            panic!("No monitor with id {} found", monitor_id);
        });
    let monitor_logical_dimensions = monitor
        .get_dimensions()
        .to_logical(monitor.get_hidpi_factor());
    info!(
        "Found monitor {} with logical dimensions: {:?}",
        monitor_id,
        (
            monitor_logical_dimensions.width,
            monitor_logical_dimensions.height
        )
    );

    //
    info!("Creating window and drawing context");
    //
    let window_builder = glutin::WindowBuilder::new()
        .with_resizable(false)
        // TODO(JaSc): Allow windowed mode when https://github.com/tomaka/winit/issues/574
        //             is fixed. Grabbing the cursor in windowed mode and ALT-TABBING in and out
        //             is currently broken.
        .with_fullscreen(Some(monitor))
        .with_title("Pongi".to_string());
    let context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
        .with_vsync(true);
    let (window, mut device, mut factory, frame_buffer, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &events_loop);

    //
    info!("Creating command buffer and pipeline state object");
    //
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
    let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();
    let pipeline_state_object = factory
        .create_pipeline_simple(&vertex_shader, &fragment_shader, pipe::new())
        .expect("Failed to create a pipeline state object");

    //
    info!("Creating pipeline data object with dummy texture and empty vertexbuffer");
    //
    use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
    let sampler_info = SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile);
    let atlas_texture = debug_load_texture(&mut factory);
    let texture_sampler = factory.create_sampler(sampler_info);
    let mut pipeline_data = pipe::Data {
        vertex_buffer: factory.create_vertex_buffer(&[]),
        texture: (atlas_texture, texture_sampler),
        target: frame_buffer,
    };

    //
    info!("Creating dummy scene");
    //
    let mut render_context = Rendercontext::new();
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let pos = (rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0));
        let dim = (rng.gen_range(0.01, 0.3), rng.gen_range(0.01, 0.3));
        let color: [f32; 4] = [
            rng.gen_range(0.2, 0.9),
            rng.gen_range(0.2, 0.9),
            rng.gen_range(0.2, 0.9),
            1.0,
        ];
        render_context.add_quad(pos, dim, color);
    }

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
                        if window_entered_fullscreen {
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
                            &mut pipeline_data.target,
                            &mut main_depth,
                        );
                        window_dimensions = (new_dim.width as f32, new_dim.height as f32);

                        // Grab mouse cursor in window
                        // NOTE: Due to https://github.com/tomaka/winit/issues/574 we need to first
                        // make sure that our resized window now spans the full screen before we
                        // allow grabbing the mouse cursor.
                        // TODO(JaSc): Remove workaround when upstream is fixed
                        if new_dim == monitor_logical_dimensions {
                            // Our window now has its final size, we can safely grab the cursor now
                            info!("Mouse cursor grabbed");
                            window_entered_fullscreen = true;
                            window.grab_cursor(true).unwrap();
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let cursor_x = position.x as f32 / window_dimensions.0;
                        let cursor_y = position.y as f32 / window_dimensions.1;
                        cursor_pos = (2.0 * cursor_x - 1.0, -2.0 * cursor_y + 1.0);
                    }
                    _ => (),
                }
            }
        });

        // Prepare vertex data
        let aspect_ratio = (window_dimensions.0 as f32) / (window_dimensions.1 as f32);
        let (vertices, indices) = render_context.get_vertices_indices(aspect_ratio, cursor_pos);
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertices, &*indices);
        pipeline_data.vertex_buffer = vertex_buffer;

        // Draw and refresh
        const BACKGROUND_COLOR: [f32; 4] = [0.7, 0.4, 0.2, 1.0];
        encoder.clear(&pipeline_data.target, BACKGROUND_COLOR);
        encoder.draw(&slice, &pipeline_state_object, &pipeline_data);
        encoder.flush(&mut device);
        window.swap_buffers().expect("Failed to swap framebuffers");
        device.cleanup();
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

#[derive(Debug)]
struct Rendercontext {
    quads: Vec<Quad>,
}

impl Rendercontext {
    fn new() -> Self {
        Rendercontext { quads: vec![] }
    }

    fn add_quad(&mut self, pos: (f32, f32), dim: (f32, f32), color: [f32; 4]) {
        self.quads.push(Quad { pos, dim, color });
    }

    fn get_vertices_indices(
        &self,
        aspect_ratio: f32,
        cursor_pos: (f32, f32),
    ) -> (Vec<Vertex>, Vec<u16>) {
        let (mut vertices, mut indices) = (vec![], vec![]);

        // Fill vertices and indices arrays with quads
        for (quad_index, quad) in self.quads.iter().enumerate() {
            quad.append_vertices_indices(
                (quad_index) as u16,
                aspect_ratio,
                &mut vertices,
                &mut indices,
            );
        }

        // Add dummy quad for cursor
        const CURSOR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        let cursor_quad = Quad {
            pos: cursor_pos,
            dim: (0.02, 0.02),
            color: CURSOR_COLOR,
        };
        cursor_quad.append_vertices_indices(
            self.quads.len() as u16,
            aspect_ratio,
            &mut vertices,
            &mut indices,
        );

        (vertices, indices)
    }
}

#[derive(Debug, Clone, Copy)]
struct Quad {
    pos: (f32, f32),
    dim: (f32, f32),
    color: [f32; 4],
}

impl Quad {
    fn append_vertices_indices(
        &self,
        quad_index: u16,
        ratio: f32,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u16>,
    ) {
        let pos = self.pos;
        let (half_width, half_height) = if ratio > 1.0 {
            (self.dim.0 / ratio, self.dim.1)
        } else {
            (self.dim.0, self.dim.1 * ratio)
        };

        vertices.extend(&[
            Vertex {
                pos: [pos.0 + half_width, pos.1 - half_height],
                uv: [1.0, 0.0],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 - half_width, pos.1 - half_height],
                uv: [0.0, 0.0],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 - half_width, pos.1 + half_height],
                uv: [0.0, 1.0],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 + half_width, pos.1 + half_height],
                uv: [1.0, 1.0],
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
