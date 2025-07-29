pub mod error;
pub mod taa;
pub mod uniforms;
pub mod camera_jitter;

use awsm_renderer_core::{
    pipeline::{fragment::ColorTargetState, primitive::PrimitiveState},
    sampler::{FilterMode, SamplerDescriptor},
    texture::TextureFormat,
};
use glam::Mat4;

use crate::{
    bind_groups::material_textures::MaterialBindGroupLayoutKey,
    materials::{post_process::PostProcessMaterial, Material, MaterialKey},
    pipeline::{
        PipelineLayoutCacheKey, PipelineLayoutKey, RenderPipelineCacheKey, RenderPipelineKey,
    },
    render::{
        post_process::{camera_jitter::PostProcessCameraJitter, error::AwsmPostProcessError, uniforms::PostProcessUniforms}, RenderContext
    },
    shaders::{
        fragment::{
            cache_key::ShaderCacheKeyFragment,
            entry::post_process::ShaderCacheKeyFragmentPostProcess,
        },
        vertex::ShaderCacheKeyVertex,
        ShaderCacheKey, ShaderKey,
    },
    textures::SamplerKey,
    AwsmRenderer,
};

pub struct PostProcess {
    pub settings: PostProcessSettings,
    pub uniforms: PostProcessUniforms,
    // only optional due to bootstrapping, it will be set before AwsmRenderer is created
    inner: Option<PostProcessInner>,
}

impl PostProcess {
    pub fn new(settings: PostProcessSettings) -> Self {
        Self {
            settings,
            uniforms: PostProcessUniforms::new(),
            inner: None,
        }
    }

    pub fn render_pipeline_key(&self) -> RenderPipelineKey {
        self.inner.as_ref().unwrap().render_pipeline_key
    }

    pub fn push_commands(
        &self,
        ctx: &mut RenderContext,
        ping_pong: bool,
    ) -> crate::error::Result<()> {
        self.inner
            .as_ref()
            .unwrap()
            .push_commands(ctx, ping_pong)
    }

    pub fn apply_camera_jitter(&mut self, projection: &mut Mat4, screen_width: u32, screen_height: u32) {
        self.inner.as_mut().unwrap().camera_jitter.apply(projection, screen_width, screen_height);
    }
}

impl AwsmRenderer {
    // so that we don't need to make the render function async
    // this is only called once, when the renderer is (re)created
    // or when the post process settings change in a way that requires recreating the render pipeline
    // such as changing the shader or the texture format
    pub async fn post_process_init(&mut self) -> crate::error::Result<()> {
        // uses cache
        let shader_key = self
            .add_shader(self.post_process.settings.shader_cache_key())
            .await?;

        // uses cache
        let linear_sampler_key = self.add_material_post_process_scene_sampler(
            self.post_process.settings.linear_sampler_descriptor(),
        )?;

        let material_key_ping = self
            .materials
            .insert(Material::PostProcess(self.post_process.settings.material()));

        let material_key_pong = self
            .materials
            .insert(Material::PostProcess(self.post_process.settings.material()));

        // uses cache
        let material_bind_group_layout_key =
            self.add_material_post_process_bind_group_layout(material_key_ping)?;
        self.add_material_post_process_bind_group_layout(material_key_pong)?;

        // uses cache
        let pipeline_layout_key = self.add_pipeline_layout(
            Some("post process"),
            self.post_process
                .settings
                .pipeline_layout_cache_key(material_bind_group_layout_key),
        )?;


        // uses cache
        let render_pipeline_key= self
            .add_render_pipeline(
                Some("post process"),
                self.post_process.settings.render_pipeline_cache_key(
                    shader_key,
                    pipeline_layout_key,
                    self.gpu.current_context_format(),
                    self.render_textures.formats.accumulation
                ),
            )
            .await?;

        self.post_process.inner = Some(PostProcessInner {
            material_key_ping,
            material_key_pong,
            linear_sampler_key,
            render_pipeline_key,
            camera_jitter: PostProcessCameraJitter::new(),
        });

        Ok(())
    }

    // this is only called when the screen size changes
    pub fn post_process_update_view(&mut self) -> crate::error::Result<()> {
        let (texture_views, _) = self.render_textures.views(&self.gpu)?;
        // safe - guaranteed to be initialized by post_process_init
        let (material_key_ping, material_key_pong, linear_sampler) = {
            let post_process = self.post_process.inner.as_ref().unwrap();
            let linear_sampler = self
                .textures
                .get_sampler(post_process.linear_sampler_key)
                .ok_or(AwsmPostProcessError::MissingPostProcessSampler(
                    post_process.linear_sampler_key,
                ))?
                .clone();

            (post_process.material_key_ping, post_process.material_key_pong, linear_sampler)
        };

        self.add_material_post_process_bind_group(material_key_ping, material_key_pong, &texture_views, linear_sampler)?;

        Ok(())
    }
}

struct PostProcessInner {
    pub material_key_ping: MaterialKey,
    pub material_key_pong: MaterialKey,
    pub linear_sampler_key: SamplerKey,
    pub render_pipeline_key: RenderPipelineKey,
    pub camera_jitter: PostProcessCameraJitter
}

impl PostProcessInner {
    pub fn push_commands(
        &self,
        ctx: &mut RenderContext,
        ping_pong: bool,
    ) -> crate::error::Result<()> {

        let material_key = if ping_pong {
            self.material_key_pong
        } else {
            self.material_key_ping
        };

        let material_bind_group = ctx
            .bind_groups
            .material_textures
            .gpu_bind_group_by_material(material_key)?;


        ctx.render_pass
            .set_bind_group(0, material_bind_group, None)?;
        // No vertex buffer needed!
        ctx.render_pass.draw(3); // Draw 3 vertices to form a triangle
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostProcessSettings {
    pub enabled: bool,
    pub tonemapping: Option<ToneMapping>,
    pub gamma_correction: bool,
    pub anti_aliasing: bool,
}

impl Default for PostProcessSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tonemapping: None,
            gamma_correction: true,
            anti_aliasing: true,
        }
    }
}

impl PostProcessSettings {
    pub fn shader_cache_key(&self) -> ShaderCacheKey {
        ShaderCacheKey::new(
            ShaderCacheKeyVertex::Quad,
            ShaderCacheKeyFragment::PostProcess(ShaderCacheKeyFragmentPostProcess {
                gamma_correction: self.gamma_correction,
                tonemapping: self.tonemapping,
                anti_aliasing: self.anti_aliasing,
            }),
        )
    }

    pub fn material(&self) -> PostProcessMaterial {
        PostProcessMaterial {}
    }

    pub fn linear_sampler_descriptor(&self) -> SamplerDescriptor<'static> {
        SamplerDescriptor {
            label: Some("post process linear sampler"),
            min_filter: Some(FilterMode::Linear),
            mag_filter: Some(FilterMode::Linear),
            ..SamplerDescriptor::default()
        }
    }

    pub fn pipeline_layout_cache_key(
        &self,
        material_bind_group_layout_key: MaterialBindGroupLayoutKey,
    ) -> PipelineLayoutCacheKey {
        PipelineLayoutCacheKey::new_post_process(material_bind_group_layout_key)
    }

    pub fn render_pipeline_cache_key(
        &self,
        shader_key: ShaderKey,
        pipeline_layout_key: PipelineLayoutKey,
        render_texture_format: TextureFormat,
        accumulation_texture_format: TextureFormat,
    ) -> RenderPipelineCacheKey {
        let primitive_state = PrimitiveState::new()
            .with_topology(web_sys::GpuPrimitiveTopology::TriangleList)
            .with_cull_mode(web_sys::GpuCullMode::None)
            .with_front_face(web_sys::GpuFrontFace::Ccw);
        RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
            .with_push_fragment_target(ColorTargetState::new(render_texture_format))
            .with_push_fragment_target(ColorTargetState::new(accumulation_texture_format))
            .with_primitive(primitive_state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToneMapping {
    KhronosPbrNeutral,
    Agx,
    Filmic,
}
