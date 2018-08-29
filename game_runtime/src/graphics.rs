use game_lib;
use game_lib::{
    Color, ComponentBytes, DrawCommand, FramebufferInfo, FramebufferTarget, Mat4, Mat4Helper, Mesh,
    Pixel, Rect, TextureArrayInfo, Vertex, VertexIndex,
};

use OptionHelper;

use gfx;
use gfx::traits::FactoryExt;
use std::collections::HashMap;

use failure;
use failure::{Error, ResultExt};

use std;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

type TextureSampler<R> = gfx::handle::Sampler<R>;
type VertexBuffer<R> = gfx::handle::Buffer<R, VertexGFX>;
type RenderTargetColor<R> = gfx::handle::RenderTargetView<R, ColorFormat>;
type RenderTargetDepth<R> = gfx::handle::DepthStencilView<R, DepthFormat>;
type ShaderResourceView<R> = gfx::handle::ShaderResourceView<R, [f32; 4]>;
type PipelineStateObject<R> = gfx::PipelineState<R, pipe::Meta>;

use gfx::state::{Blend, BlendValue, ColorMask, Equation, Factor};
gfx_defines! {
    vertex VertexGFX {
        pos: [f32; 4] = "a_Pos",
        uv: [f32; 3] = "a_Uv",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vertex_buffer: gfx::VertexBuffer<VertexGFX> = (),

        transform: gfx::Global<[[f32; 4];4]> = "u_Transform",
        use_texture_array: gfx::Global<i32> = "u_UseTextureArray",

        texture: gfx::TextureSampler<[f32; 4]> = "u_Sampler",
        texture_array: gfx::TextureSampler<[f32; 4]> = "u_SamplerArray",

        out_color: gfx::BlendTarget<ColorFormat> = ("Target0",
                                                    ColorMask::all(),
                                                    Blend::new(
                                                        Equation::Add,
                                                        Factor::One,
                                                        Factor::OneMinus(BlendValue::SourceAlpha)
                                                        )
                                                   ),
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
        trace!("Creating rasterizers for line drawing and filling");
        //
        use gfx::state::{CullFace, FrontFace, RasterMethod, Rasterizer};
        let line_rasterizer = Rasterizer {
            front_face: FrontFace::CounterClockwise,
            cull_face: CullFace::Nothing,
            method: RasterMethod::Line(1),
            offset: None,
            samples: None,
        };
        let fill_rasterizer = Rasterizer {
            front_face: FrontFace::CounterClockwise,
            cull_face: CullFace::Nothing,
            method: RasterMethod::Fill,
            offset: None,
            samples: None,
        };

        //
        trace!("Creating framebuffer pipeline state objects");
        //
        let pipeline_state_object_line = factory
            .create_pipeline_state(
                &shader_set,
                gfx::Primitive::LineList,
                line_rasterizer,
                pipe::new(),
            )
            .context("Failed to create pipeline state object for line drawing")?;

        let pipeline_state_object_fill = factory
            .create_pipeline_state(
                &shader_set,
                gfx::Primitive::TriangleList,
                fill_rasterizer,
                pipe::new(),
            )
            .context("Failed to create pipeline state object for filled drawing")?;

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
        transform: &Mat4,
        texture: ShaderResourceView<R>,
        texture_mode: TextureMode,
        vertex_buffer: VertexBuffer<R>,
    ) -> pipe::Data<R>
    where
        R: gfx::Resources,
    {
        pipe::Data {
            vertex_buffer,

            transform: (*transform).into(),
            use_texture_array: match texture_mode {
                TextureMode::Regular => 0,
                TextureMode::ArrayTexture => 1,
            },

            texture: (texture.clone(), self.texture_sampler.clone()),
            texture_array: (texture, self.texture_sampler.clone()),

            out_color: self.color_render_target_view.clone(),
            out_depth: self.depth_render_target_view.clone(),
        }
    }
}

//==================================================================================================
// RenderingContext
//==================================================================================================
//

#[derive(Debug, Copy, Clone)]
enum TextureMode {
    ArrayTexture,
    Regular,
}

#[derive(Debug, Copy, Clone)]
pub enum DrawMode {
    Lines,
    Fill,
}

pub struct RenderingContext<C, R, F>
where
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    F: gfx::Factory<R>,
{
    factory: F,
    // NOTE: `encoder` and `screen_framebuffer` need to be public for direct access from
    //        gfx_glutin in the mainloop
    // TODO(JaSc): Evaluate if we can contain this with accessor functions
    pub encoder: gfx::Encoder<R, C>,
    pub screen_framebuffer: Framebuffer<R>,

    framebuffers: HashMap<FramebufferInfo, Framebuffer<R>>,
    textures: HashMap<TextureArrayInfo, ShaderResourceView<R>>,
    textures_pixeldata: HashMap<TextureArrayInfo, Vec<Vec<Pixel>>>,
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

    pub fn update_screen_dimensions(&mut self, width: u16, height: u16) {
        let old_screen_frambuffer_info = self.screen_framebuffer.info.clone();
        self.screen_framebuffer.info = FramebufferInfo {
            id: old_screen_frambuffer_info.id,
            width,
            height,
            name: old_screen_frambuffer_info.name,
        }
    }

    pub fn process_draw_commands(&mut self, draw_commands: Vec<DrawCommand>) -> Result<(), Error> {
        trace!("Processing {:?} draw commands", draw_commands.len());

        for mut draw_command in draw_commands {
            let draw_command = &mut draw_command;
            let processing_result = match draw_command {
                DrawCommand::DrawLines {
                    transform,
                    mesh,
                    texture_array_info,
                    framebuffer,
                } => {
                    let (vertices, indices) = mesh.to_vertices_indices();
                    self.draw(
                        transform,
                        self.get_texture_array(&texture_array_info)?.clone(),
                        TextureMode::ArrayTexture,
                        vertices,
                        indices,
                        framebuffer,
                        DrawMode::Lines,
                    )
                }
                DrawCommand::DrawPolys {
                    transform,
                    mesh,
                    texture_array_info,
                    framebuffer,
                } => {
                    let (vertices, indices) = mesh.to_vertices_indices();
                    self.draw(
                        transform,
                        self.get_texture_array(&texture_array_info)?.clone(),
                        TextureMode::ArrayTexture,
                        vertices,
                        indices,
                        framebuffer,
                        DrawMode::Fill,
                    )
                }
                DrawCommand::Clear {
                    framebuffer,
                    color,
                    depth,
                } => self.clear(&framebuffer, *color, *depth),
                DrawCommand::ClearColor { framebuffer, color } => {
                    self.clear_color(&framebuffer, *color)
                }
                DrawCommand::ClearDepth { framebuffer, depth } => {
                    self.clear_depth(&framebuffer, *depth)
                }
                DrawCommand::BlitFramebuffer {
                    source_framebuffer,
                    target_framebuffer,
                    source_rect,
                    target_rect,
                } => self.blit_framebuffer(
                    source_framebuffer,
                    target_framebuffer,
                    *source_rect,
                    *target_rect,
                ),
                DrawCommand::CreateFramebuffer { framebuffer_info } => {
                    self.create_framebuffer(framebuffer_info)
                }
                DrawCommand::DeleteFramebuffer { framebuffer_info } => {
                    self.delete_framebuffer(framebuffer_info)
                }
                DrawCommand::CreateTextureArray {
                    texture_array_info,
                    pixels,
                } => {
                    // NOTE: We take the pixeldata out of the drawcommand as we need ownership
                    let taken_pixels = std::mem::replace(pixels, Vec::new());
                    self.create_texture_array(texture_array_info, taken_pixels)
                }
                DrawCommand::DeleteTextureArray { texture_array_info } => {
                    self.delete_texture_array(&texture_array_info)
                }
            };
            processing_result
                .context(format!("Could not execute draw command {:?}", draw_command))?;
        }
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Drawing
    //
    fn draw(
        &mut self,
        transform: &Mat4,
        texture: ShaderResourceView<R>,
        texture_mode: TextureMode,
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
        let pipeline_data =
            framebuffer.create_pipeline_data(transform, texture, texture_mode, vertex_buffer);

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
        clear_depth: f32,
    ) -> Result<(), Error> {
        self.clear_color(framebuffer_target, clear_color)?;
        self.clear_depth(framebuffer_target, clear_depth)?;

        Ok(())
    }

    fn clear_color(
        &mut self,
        framebuffer_target: &FramebufferTarget,
        clear_color: Color,
    ) -> Result<(), Error> {
        let target = self.get_framebuffer(framebuffer_target)?;
        self.encoder
            .clear(&target.color_render_target_view, clear_color.into());

        Ok(())
    }

    fn clear_depth(
        &mut self,
        framebuffer_target: &FramebufferTarget,
        clear_depth: f32,
    ) -> Result<(), Error> {
        let target = self.get_framebuffer(framebuffer_target)?;

        self.encoder
            .clear_depth(&target.depth_render_target_view, clear_depth);

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
        let vertices = game_lib::vertices_from_rects(
            target_rect,
            Rect::unit_rect(),
            0,
            0.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        let indices: [VertexIndex; 6] = [0, 1, 2, 2, 3, 0];

        // NOTE: The projection matrix is flipped 'upside-down' for correct blitting
        let projection_mat = Mat4::ortho_origin_bottom_left(
            f32::from(target_framebuffer_info.width),
            f32::from(target_framebuffer_info.height),
            -1.0,
            1.0,
        );

        let texture = source_framebuffer.shader_resource_view.ok_or_else(|| {
            failure::err_msg(format!(
            "Could not blit framebuffer because source {:?} does not have a shader resouce view",
            source_framebuffer_info
        ))
        })?;

        self.draw(
            &projection_mat,
            texture,
            TextureMode::Regular,
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

        self.framebuffers.remove(&framebuffer_info).ok_or_else(|| {
            failure::err_msg(format!(
                "Could not delete framebuffer because it did not exist for {:?}",
                framebuffer_info
            ))
        })?;

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
            .ok_or_else(|| {
                failure::err_msg(format!(
                    "Could not find framebuffer for {:?}",
                    framebuffer_info
                ))
            })?
            .clone())
    }

    // ---------------------------------------------------------------------------------------------
    // Textures
    //
    fn create_texture_array(
        &mut self,
        texture_array_info: &TextureArrayInfo,
        pixels: Vec<Vec<Pixel>>,
    ) -> Result<(), Error> {
        debug!("Creating texture for {:?}", texture_array_info);

        let kind = gfx::texture::Kind::D2Array(
            texture_array_info.width,
            texture_array_info.height,
            texture_array_info.num_textures,
            gfx::texture::AaMode::Single,
        );
        // TODO(JaSc): Check if we can find a easiert/faster solution for deconstructing a
        //             Vec<Vec<Pixel>> into a &[&[u8]]
        let data: Vec<_> = pixels.iter().map(|texture| texture.as_bytes()).collect();
        let (_, view) = self
            .factory
            .create_texture_immutable_u8::<ColorFormat>(kind, gfx::texture::Mipmap::Provided, &data)
            .context(format!(
                "Could not create texture data from pixels for {:?}",
                texture_array_info
            ))?;

        self.textures
            .insert(texture_array_info.clone(), view)
            .none_or(failure::err_msg(format!(
                "Could not create texture because it already exists for {:?}",
                texture_array_info
            )))?;

        self.textures_pixeldata
            .insert(texture_array_info.clone(), pixels)
            .none_or(failure::err_msg(format!(
                "Could not create pixeldata cache because it already exists for {:?}",
                texture_array_info
            )))?;

        Ok(())
    }

    fn delete_texture_array(&mut self, texture_array_info: &TextureArrayInfo) -> Result<(), Error> {
        debug!("Deleting texture for {:?}", texture_array_info);

        self.textures.remove(texture_array_info).ok_or_else(|| {
            failure::err_msg(format!(
                "Could not delete texture because it did not exist for {:?}",
                texture_array_info
            ))
        })?;
        self.textures_pixeldata
            .remove(texture_array_info)
            .ok_or_else(|| {
                failure::err_msg(format!(
                    "Could not delete pixeldata cache because it did not exist for {:?}",
                    texture_array_info
                ))
            })?;
        Ok(())
    }

    fn get_texture_array(
        &self,
        texture_array_info: &TextureArrayInfo,
    ) -> Result<&ShaderResourceView<R>, Error> {
        Ok(self.textures.get(texture_array_info).ok_or_else(|| {
            failure::err_msg(format!(
                "Could not find texture for {:?}",
                texture_array_info
            ))
        })?)
    }

    fn _get_texture_array_pixeldata(
        &self,
        texture_array_info: &TextureArrayInfo,
    ) -> Result<&Vec<Vec<Pixel>>, Error> {
        Ok(self
            .textures_pixeldata
            .get(texture_array_info)
            .ok_or_else(|| {
                failure::err_msg(format!(
                    "Could not find pixeldata for {:?}",
                    texture_array_info
                ))
            })?)
    }
}
