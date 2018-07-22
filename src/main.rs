#[macro_use]
extern crate gfx;
extern crate gfx_window_sdl;
extern crate rand;
extern crate sdl2;

use rand::prelude::*;

use gfx::traits::FactoryExt;
use gfx::Device;
use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::Keycode;

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
    // Init SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.gl_set_swap_interval(sdl2::video::SwapInterval::Immediate);

    // Configure OpenGl
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 2);

    // Create window and OpenGL context
    let mut window_builder = video_subsystem.window("Pongi", 1280, 720);
    window_builder
        //.resizable()
        .fullscreen_desktop()
        .input_grabbed()
        .position_centered();
    let (window, _gl_context, mut device, mut factory, main_color, mut main_depth) =
        gfx_window_sdl::init::<ColorFormat, DepthFormat>(&video_subsystem, window_builder).unwrap();
    sdl_context.mouse().show_cursor(false);

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
    let mut events = sdl_context.event_pump().unwrap();

    // Begin mainloop
    while running {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => match keycode {
                    Keycode::Escape => running = false,
                    _ => (),
                },
                Event::Window {
                    win_event: WindowEvent::Resized(..),
                    ..
                } => {
                    gfx_window_sdl::update_views(&window, &mut pipeline_data.out, &mut main_depth);
                }
                Event::MouseMotion { x, y, .. } => {
                    let (window_width, window_height) = window.size();
                    let cursor_x = x as f32 / window_width as f32;
                    let cursor_y = y as f32 / window_height as f32;
                    cursor_pos = (2.0 * cursor_x - 1.0, -2.0 * cursor_y + 1.0);
                }
                _ => (),
            }
        }

        // Determine aspect ratio and create vertices for all squares
        let (window_width, window_height) = window.size();
        let aspect_ratio = (window_width as f32) / (window_height as f32);
        let (vertices, indices) = render_context.get_vertices_indices(aspect_ratio, cursor_pos);
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertices, &*indices);
        pipeline_data.vbuf = vertex_buffer;

        // Draw squares and swap framebuffers
        const BACKGROUND_COLOR: [f32; 4] = [0.7, 0.4, 0.2, 1.0];
        encoder.clear(&pipeline_data.out, BACKGROUND_COLOR);
        encoder.draw(&slice, &pipeline_state_object, &pipeline_data);
        encoder.flush(&mut device);
        window.gl_swap_window();
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
