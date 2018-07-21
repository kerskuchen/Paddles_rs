#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate rand;

use rand::prelude::*;

use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::{Event, GlContext, KeyboardInput, WindowEvent};

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let monitor = Some(
        events_loop
            .get_available_monitors()
            .nth(1)
            .expect("No monitor found"),
    );
    // NOTE: Uncomment the following line to switch to windowed mode
    //let monitor = None;

    // Create window and drawing context
    let window_builder = glutin::WindowBuilder::new()
        .with_title("Pongi".to_string())
        .with_fullscreen(monitor)
        .with_dimensions((1280, 720).into());
    let context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
        .with_vsync(true);
    let (window, mut device, mut factory, main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &events_loop);

    // Create create command buffer and pipeline state object
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
    let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();
    let pipeline_state_object = factory
        .create_pipeline_simple(&vertex_shader, &fragment_shader, pipe::new())
        .expect("Failed to create a pipeline state object");

    // Create pipeline data with empty vertexbuffer
    let mut pipeline_data = pipe::Data {
        vbuf: factory.create_vertex_buffer(&[]),
        out: main_color,
    };

    // Create scene
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

    // State variables
    let mut running = true;
    let mut cursor_pos = (0.0, 0.0);
    let mut window_dimensions: (f32, f32) = (0.0, 0.0);

    // Begin mainloop
    while running {
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
                    WindowEvent::Resized(new_dim) => {
                        window.resize(new_dim.to_physical(window.get_hidpi_factor()));
                        gfx_window_glutin::update_views(
                            &window,
                            &mut pipeline_data.out,
                            &mut main_depth,
                        );
                        window_dimensions = (new_dim.width as f32, new_dim.height as f32);
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

        let aspect_ratio = (window_dimensions.0 as f32) / (window_dimensions.1 as f32);
        let (vertices, indices) = render_context.get_vertices_indices(aspect_ratio, cursor_pos);
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertices, &*indices);
        pipeline_data.vbuf = vertex_buffer;

        const BACKGROUND_COLOR: [f32; 4] = [0.7, 0.4, 0.2, 1.0];
        encoder.clear(&pipeline_data.out, BACKGROUND_COLOR);
        encoder.draw(&slice, &pipeline_state_object, &pipeline_data);

        encoder.flush(&mut device);
        window.swap_buffers().expect("Failed to swap framebuffers");
        device.cleanup();
    }
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
                color: self.color,
            },
            Vertex {
                pos: [pos.0 - half_width, pos.1 - half_height],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 - half_width, pos.1 + half_height],
                color: self.color,
            },
            Vertex {
                pos: [pos.0 + half_width, pos.1 + half_height],
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
