use game_lib::{
    Color, ComponentBytes, Mat4, Mat4Helper, Pixel, Point, Quad, Rect, SquareMatrix, Texture,
    Vertex, VertexIndex,
};

use gfx;
use gfx::traits::FactoryExt;
use std::collections::HashMap;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

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
    pub encoder: gfx::Encoder<R, C>,

    pub screen_pipeline_data: pipe::Data<R>,
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
        vertices: &[Vertex],
        indices: &[VertexIndex],
    ) {
        let (canvas_vertex_buffer, canvas_slice) = self
            .factory
            .create_vertex_buffer_with_slice(convert_to_gfx_format(&vertices), &*indices);

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
        vertices: &[Vertex],
        indices: &[VertexIndex],
    ) {
        let (canvas_vertex_buffer, canvas_slice) = self
            .factory
            .create_vertex_buffer_with_slice(convert_to_gfx_format(&vertices), &*indices);

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

    pub fn add_texture(&mut self, texture: Texture, pixels: Vec<Pixel>) {
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

/// Converts a slice of [`Vertex`] into a slice of [`VertexGFX`] for gfx to consume.
///
/// Note that both types are memory-equivalent so the conversion is just a transmutation
pub fn convert_to_gfx_format(vertices: &[Vertex]) -> &[VertexGFX] {
    unsafe { &*(vertices as *const [Vertex] as *const [VertexGFX]) }
}
