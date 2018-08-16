use game_lib::{
    Color, ComponentBytes, DrawCommand, DrawMode, FramebufferInfo, FramebufferTarget, Mat4,
    Mat4Helper, Pixel, Quad, Rect, TextureInfo, Vertex, VertexIndex,
};

use OptionHelper;

use gfx;
use gfx::traits::FactoryExt;
use std::collections::HashMap;

use failure;
use failure::{Error, ResultExt};

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

type TextureSampler<R> = gfx::handle::Sampler<R>;
type VertexBuffer<R> = gfx::handle::Buffer<R, VertexGFX>;
type RenderTargetColor<R> = gfx::handle::RenderTargetView<R, ColorFormat>;
type RenderTargetDepth<R> = gfx::handle::DepthStencilView<R, DepthFormat>;
type ShaderResourceView<R> = gfx::handle::ShaderResourceView<R, [f32; 4]>;
type PipelineStateObject<R> = gfx::PipelineState<R, pipe::Meta>;

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
// Framebuffer
//==================================================================================================
//

#[derive(Clone)]
pub struct Framebuffer<R>
where
    R: gfx::Resources,
{
    pub info: FramebufferInfo,
    pub texture_sampler: TextureSampler<R>,

    pub pipeline_state_object_fill: PipelineStateObject<R>,
    pub pipeline_state_object_line: PipelineStateObject<R>,

    pub color_render_target_view: RenderTargetColor<R>,
    pub depth_render_target_view: RenderTargetDepth<R>,
    pub shader_resource_view: Option<ShaderResourceView<R>>,
}

impl<R> Framebuffer<R>
where
    R: gfx::Resources,
{
    fn from_render_targets<F>(
        factory: &mut F,
        framebuffer_id: u32,
        framebuffer_name: &str,
        color_render_target_view: RenderTargetColor<R>,
        depth_render_target_view: RenderTargetDepth<R>,
        shader_resource_view: Option<ShaderResourceView<R>>,
    ) -> Result<Framebuffer<R>, Error>
    where
        F: gfx::Factory<R>,
    {
        info!("Creating framebuffer {:?}", &framebuffer_name);
        let info = FramebufferInfo {
            id: framebuffer_id,
            width: color_render_target_view.get_dimensions().0,
            height: color_render_target_view.get_dimensions().1,
            name: String::from(framebuffer_name),
        };

        //
        trace!("Creating default nearest neighbour texture sampler");
        //
        use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
        let sampler_info = SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile);
        let texture_sampler = factory.create_sampler(sampler_info);

        //
        trace!("Creating shader set for framebuffer");
        //
        let vertex_shader = include_bytes!("shaders/basic.glslv").to_vec();
        let fragment_shader = include_bytes!("shaders/basic.glslf").to_vec();
        let shader_set = factory
            .create_shader_set(&vertex_shader, &fragment_shader)
            .context("Could not create shader set for framebuffer")?;

        //
        trace!("Creating framebuffer pipeline state object for line drawing");
        //
        use gfx::state::{CullFace, FrontFace, RasterMethod, Rasterizer};
        let line_rasterizer = Rasterizer {
            front_face: FrontFace::CounterClockwise,
            cull_face: CullFace::Nothing,
            method: RasterMethod::Line(1),
            offset: None,
            samples: None,
        };
        let pipeline_state_object_line = factory
            .create_pipeline_state(
                &shader_set,
                gfx::Primitive::LineList,
                line_rasterizer,
                pipe::new(),
            )
            .context("Failed to create framebuffer pipeline state object for line drawing")?;

        //
        trace!("Creating framebuffer pipeline state object for filled polygon drawing");
        //
        let fill_rasterizer = Rasterizer {
            front_face: FrontFace::CounterClockwise,
            cull_face: CullFace::Nothing,
            method: RasterMethod::Fill,
            offset: None,
            samples: None,
        };
        let pipeline_state_object_fill = factory
            .create_pipeline_state(
                &shader_set,
                gfx::Primitive::TriangleList,
                fill_rasterizer,
                pipe::new(),
            )
            .context("Failed to create framebuffer pipeline state object for filled drawing")?;

        Ok(Framebuffer {
            info,
            texture_sampler,

            pipeline_state_object_fill,
            pipeline_state_object_line,

            shader_resource_view,
            color_render_target_view,
            depth_render_target_view,
        })
    }

    fn new<F>(factory: &mut F, info: &FramebufferInfo) -> Result<Framebuffer<R>, Error>
    where
        F: gfx::Factory<R>,
    {
        //
        info!("Creating offscreen render targets");
        //
        let (_, shader_resource_view, color_render_target_view) = factory
            .create_render_target::<ColorFormat>(info.width, info.height)
            .context("Failed to create a framebuffer color render target")?;
        let depth_render_target_view = factory
            .create_depth_stencil_view_only::<DepthFormat>(info.width, info.height)
            .context("Failed to create a framebuffer depth render target")?;

        Framebuffer::from_render_targets(
            factory,
            info.id,
            &info.name,
            color_render_target_view,
            depth_render_target_view,
            Some(shader_resource_view),
        )
    }

    fn create_pipeline_data(
        &self,
        transform: Mat4,
        texture: ShaderResourceView<R>,
        vertex_buffer: VertexBuffer<R>,
    ) -> pipe::Data<R>
    where
        R: gfx::Resources,
    {
        pipe::Data {
            vertex_buffer,
            texture: (texture, self.texture_sampler.clone()),
            transform: transform.into(),
            out_color: self.color_render_target_view.clone(),
            out_depth: self.depth_render_target_view.clone(),
        }
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
    // NOTE: `encoder` and `screen_framebuffer` need to be public for direct access from
    //        gfx_glutin in the mainloop
    pub encoder: gfx::Encoder<R, C>,
    pub screen_framebuffer: Framebuffer<R>,

    framebuffers: HashMap<FramebufferInfo, Framebuffer<R>>,
    textures: HashMap<TextureInfo, ShaderResourceView<R>>,
    textures_pixeldata: HashMap<TextureInfo, Vec<Pixel>>,
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
        screen_color_render_target_view: RenderTargetColor<R>,
        screen_depth_render_target_view: RenderTargetDepth<R>,
    ) -> Result<RenderingContext<C, R, F>, Error> {
        info!("Creating rendering context");

        let framebuffer_id = u32::max_value();
        let framebuffer_name = "Mainscreen";
        let screen_framebuffer = Framebuffer::from_render_targets(
            &mut factory,
            framebuffer_id,
            framebuffer_name,
            screen_color_render_target_view,
            screen_depth_render_target_view,
            None,
        ).context(format!(
            "Could not create framebuffer {:?}",
            framebuffer_name
        ))?;

        Ok(RenderingContext {
            factory,
            encoder,
            screen_framebuffer,
            framebuffers: HashMap::new(),
            textures: HashMap::new(),
            textures_pixeldata: HashMap::new(),
        })
    }

    pub fn process_draw_commands(&mut self, draw_commands: Vec<DrawCommand>) -> Result<(), Error> {
        trace!("Processing {:?} draw commands", draw_commands.len());

        for draw_command in draw_commands {
            trace!("Processing draw command: {:?}", draw_command);
            match draw_command {
                DrawCommand::Draw {
                    transform,
                    vertices,
                    indices,
                    texture_info,
                    framebuffer,
                    draw_mode,
                } => {
                    self.draw(
                        transform,
                        self.get_texture(&texture_info)?.clone(),
                        &vertices,
                        &indices,
                        &framebuffer,
                        draw_mode,
                    ).context("Could not execute draw command 'Draw'")?;
                }
                DrawCommand::Clear { framebuffer, color } => {
                    self.clear(&framebuffer, color)
                        .context("Could not execute draw command 'Clear'")?;
                }
                DrawCommand::BlitFramebuffer {
                    source_framebuffer,
                    target_framebuffer,
                    source_rect,
                    target_rect,
                } => {
                    self.blit_framebuffer(
                        &source_framebuffer,
                        &target_framebuffer,
                        source_rect,
                        target_rect,
                    ).context("Could not execute draw command 'BlitFramebuffer'")?;
                }
                DrawCommand::CreateFramebuffer { framebuffer_info } => {
                    self.create_framebuffer(&framebuffer_info)
                        .context("Could not execute draw command 'CreateFramebuffer'")?;
                }
                DrawCommand::DeleteFramebuffer { framebuffer_info } => {
                    self.delete_framebuffer(&framebuffer_info)
                        .context("Could not execute draw command 'DeleteFramebuffer'")?;
                }
                DrawCommand::CreateTexture {
                    texture_info,
                    pixels,
                } => {
                    self.create_texture(&texture_info, pixels)
                        .context("Could not execute draw command 'CreateTexture'")?;
                }
                DrawCommand::DeleteTexture { texture_info } => {
                    self.delete_texture(&texture_info)
                        .context("Could not execute draw command 'DeleteTexture'")?;
                }
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Drawing
    //
    fn draw(
        &mut self,
        transform: Mat4,
        texture: ShaderResourceView<R>,
        vertices: &[Vertex],
        indices: &[VertexIndex],
        framebuffer_target: &FramebufferTarget,
        draw_mode: DrawMode,
    ) -> Result<(), Error>
    where
        R: gfx::Resources,
    {
        // TODO(JaSc): Evaluate if we can avoid cloning the framebuffer and re-creating its pipeline
        //             data from scratch. We cannot borrow it immutable/mutable from ourselves
        //             though as we already borrow encoder from ourselves mutably.
        let (vertex_buffer, slice) = self
            .factory
            .create_vertex_buffer_with_slice(convert_to_gfx_format(&vertices), &*indices);
        let framebuffer = self.get_framebuffer(framebuffer_target)?;
        let pipeline_data = framebuffer.create_pipeline_data(transform, texture, vertex_buffer);

        match draw_mode {
            DrawMode::Lines => self.encoder.draw(
                &slice,
                &framebuffer.pipeline_state_object_line,
                &pipeline_data,
            ),
            DrawMode::Fill => self.encoder.draw(
                &slice,
                &framebuffer.pipeline_state_object_fill,
                &pipeline_data,
            ),
        }
        Ok(())
    }

    fn clear(
        &mut self,
        framebuffer_target: &FramebufferTarget,
        clear_color: Color,
    ) -> Result<(), Error> {
        let target = self.get_framebuffer(framebuffer_target)?;
        self.encoder
            .clear(&target.color_render_target_view, clear_color.into());
        self.encoder
            .clear_depth(&target.depth_render_target_view, 1.0);

        Ok(())
    }

    fn blit_framebuffer(
        &mut self,
        source_framebuffer_info: &FramebufferInfo,
        target_framebuffer: &FramebufferTarget,
        source_rect: Rect,
        target_rect: Rect,
    ) -> Result<(), Error> {
        let source_framebuffer = self.get_framebuffer_by_info(source_framebuffer_info)?;
        let target_framebuffer_info = self.get_framebuffer(target_framebuffer)?.info;

        trace!(
            "Blitting framebuffer:\n  source: {:?}\n  target: {:?}\n  source: {:?}\n  target: {:?}",
            source_framebuffer_info,
            target_framebuffer_info,
            source_rect,
            target_rect
        );

        // TODO(JaSc): Incorporate source_rect into blit_quad calculation
        let blit_quad = Quad::new(target_rect, 0.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let vertices = blit_quad.into_vertices();
        let indices: [VertexIndex; 6] = [0, 1, 2, 2, 3, 0];

        // NOTE: The projection matrix is flipped upside-down for correct blitting
        let projection_mat = Mat4::ortho_bottom_left_flipped_y(
            f32::from(target_framebuffer_info.width),
            f32::from(target_framebuffer_info.height),
            0.0,
            1.0,
        );

        let texture = source_framebuffer
            .shader_resource_view
            .ok_or(failure::err_msg(format!(
            "Could not blit framebuffer because source {:?} does not have a shader resouce view",
            source_framebuffer.info
        )))?;

        self.draw(
            projection_mat,
            texture,
            &vertices,
            &indices,
            target_framebuffer,
            DrawMode::Fill,
        )?;

        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Framebuffers
    //
    fn create_framebuffer(&mut self, framebuffer_info: &FramebufferInfo) -> Result<(), Error> {
        debug!("Creating framebuffer for {:?}", framebuffer_info);

        let framebuffer = Framebuffer::new(&mut self.factory, framebuffer_info).context(format!(
            "Could not create framebuffer {:?}",
            framebuffer_info
        ))?;

        self.framebuffers
            .insert(framebuffer_info.clone(), framebuffer)
            .none_or(failure::err_msg(format!(
                "Could not create framebuffer because it already exists for {:?}",
                framebuffer_info
            )))?;

        Ok(())
    }

    fn delete_framebuffer(&mut self, framebuffer_info: &FramebufferInfo) -> Result<(), Error> {
        debug!("Deleting framebuffer for {:?}", framebuffer_info);

        self.framebuffers
            .remove(&framebuffer_info)
            .ok_or(failure::err_msg(format!(
                "Could not delete framebuffer because it did not exist for {:?}",
                framebuffer_info
            )))?;

        Ok(())
    }

    fn get_framebuffer(&self, framebuffer: &FramebufferTarget) -> Result<Framebuffer<R>, Error> {
        match framebuffer {
            FramebufferTarget::Screen => Ok(self.screen_framebuffer.clone()),
            FramebufferTarget::Offscreen(framebuffer_info) => {
                self.get_framebuffer_by_info(framebuffer_info)
            }
        }
    }

    fn get_framebuffer_by_info(
        &self,
        framebuffer_info: &FramebufferInfo,
    ) -> Result<Framebuffer<R>, Error> {
        Ok(self
            .framebuffers
            .get(framebuffer_info)
            .ok_or(failure::err_msg(format!(
                "Could not find framebuffer for {:?}",
                framebuffer_info
            )))?
            .clone())
    }

    // ---------------------------------------------------------------------------------------------
    // Textures
    //
    fn create_texture(
        &mut self,
        texture_info: &TextureInfo,
        pixels: Vec<Pixel>,
    ) -> Result<(), Error> {
        debug!("Creating texture for {:?}", texture_info);

        let kind = gfx::texture::Kind::D2(
            texture_info.width,
            texture_info.height,
            gfx::texture::AaMode::Single,
        );
        let (_, view) = self
            .factory
            .create_texture_immutable_u8::<ColorFormat>(
                kind,
                gfx::texture::Mipmap::Provided,
                &[(&pixels).as_bytes()],
            )
            .context(format!(
                "Could not create texture data from pixels for {:?}",
                texture_info
            ))?;

        self.textures
            .insert(texture_info.clone(), view)
            .none_or(failure::err_msg(format!(
                "Could not create texture because it already exists for {:?}",
                texture_info
            )))?;

        self.textures_pixeldata
            .insert(texture_info.clone(), pixels)
            .none_or(failure::err_msg(format!(
                "Could not create pixeldata cache because it already exists for {:?}",
                texture_info
            )))?;

        Ok(())
    }

    fn delete_texture(&mut self, texture_info: &TextureInfo) -> Result<(), Error> {
        debug!("Deleting texture for {:?}", texture_info);

        self.textures
            .remove(texture_info)
            .ok_or(failure::err_msg(format!(
                "Could not delete texture because it did not exist for {:?}",
                texture_info
            )))?;
        self.textures_pixeldata
            .remove(texture_info)
            .ok_or(failure::err_msg(format!(
                "Could not delete pixeldata cache because it did not exist for {:?}",
                texture_info
            )))?;
        Ok(())
    }

    fn get_texture(&self, texture_info: &TextureInfo) -> Result<&ShaderResourceView<R>, Error> {
        Ok(self
            .textures
            .get(texture_info)
            .ok_or(failure::err_msg(format!(
                "Could not find texture for {:?}",
                texture_info
            )))?)
    }

    fn _get_texture_pixeldata(&self, texture_info: &TextureInfo) -> Result<&Vec<Pixel>, Error> {
        Ok(self
            .textures_pixeldata
            .get(texture_info)
            .ok_or(failure::err_msg(format!(
                "Could not find pixeldata for {:?}",
                texture_info
            )))?)
    }
}

/// Converts a slice of [`Vertex`] into a slice of [`VertexGFX`] for gfx to consume.
///
/// Note that both types are memory-equivalent so the conversion is just a transmutation
pub fn convert_to_gfx_format(vertices: &[Vertex]) -> &[VertexGFX] {
    unsafe { &*(vertices as *const [Vertex] as *const [VertexGFX]) }
}
