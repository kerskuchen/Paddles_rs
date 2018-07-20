#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;

use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::{Event, GlContext, KeyboardInput, VirtualKeyCode, WindowEvent};

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

const BACKGROUND_COLOR: [f32; 4] = [0.7, 0.4, 0.2, 1.0];
const SQUARE_COLOR: [f32; 3] = [0.2, 0.4, 0.7];

const SQUARE_VERTICES: &[Vertex] = &[
    Vertex {
        pos: [0.5, -0.5],
        color: SQUARE_COLOR,
    },
    Vertex {
        pos: [-0.5, -0.5],
        color: SQUARE_COLOR,
    },
    Vertex {
        pos: [-0.5, 0.5],
        color: SQUARE_COLOR,
    },
    Vertex {
        pos: [0.5, 0.5],
        color: SQUARE_COLOR,
    },
];

const SQUARE_INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

gfx_defines!{
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        color: [f32; 3] = "a_Color",
    }

    pipeline pipe{
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

fn main() {
    let window_builder = glutin::WindowBuilder::new()
        .with_title("Pongi".to_string())
        .with_dimensions((1280, 720).into());

    let context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
        .with_vsync(true);

    let mut events_loop = glutin::EventsLoop::new();

    let (window, mut device, mut factory, main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &events_loop);

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
    let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();
    let pipeline_state_object = factory
        .create_pipeline_simple(&vertex_shader, &fragment_shader, pipe::new())
        .unwrap();

    let (vertex_buffer, slice) =
        factory.create_vertex_buffer_with_slice(SQUARE_VERTICES, SQUARE_INDICES);
    let mut data = pipe::Data {
        vbuf: vertex_buffer,
        out: main_color,
    };

    let mut running = true;
    while running {
        events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => running = false,
                    WindowEvent::Resized(size) => {
                        window.resize(size.to_physical(window.get_hidpi_factor()));
                        gfx_window_glutin::update_views(&window, &mut data.out, &mut main_depth);
                    }
                    _ => (),
                }
            }
        });

        encoder.clear(&data.out, BACKGROUND_COLOR);
        encoder.draw(&slice, &pipeline_state_object, &data);

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}
