use crate::{pipeline::RenderPipelineKey, render::RenderContext, shaders::ShaderKey, AwsmRenderer};


pub struct PostProcess {
    pub settings: PostProcessSettings,
    // only optional due to bootstrapping, it will be set before AwsmRenderer is created
    inner: Option<PostProcessInner>,
}

impl PostProcess {
    pub fn new(settings: PostProcessSettings) -> Self {
        Self {
            settings,
            inner: None,
        }
    }

    pub fn render_pipeline_key(&self) -> RenderPipelineKey {
        self.inner.as_ref().unwrap().render_pipeline_key
    }
}

impl AwsmRenderer {
    // so that we don't need to make the render function async
    // we initialize this 
    pub async fn post_process_init(&mut self) -> crate::error::Result<()> {
        // let texture_key = self.textures.add_texture(self.render_textures);
        // let shader_key = self.add_shader(self.post_process.settings.shader_cache_key()).await?;
        // let material_key = self.materials.insert(
        //     &self.gpu,
        //     &mut self.bind_groups,
        //     &self.textures,
        //     self.post_process.settings.material_deps(),
        // )?;
        // let material_bind_group_layout_key = self
        //     .bind_groups
        //     .material_textures
        //     .get_layout_key(material_key)?;
        // let pipeline_layout_key = self.add_pipeline_layout(Some("post process"), self.post_process.settings.pipeline_layout_cache_key(material_bind_group_layout_key))?;
        // let render_pipeline_key = self.add_render_pipeline(Some("post process"), self.post_process.settings.render_pipeline_cache_key(shader_key, pipeline_layout_key)).await?;
        // self.post_process.inner = Some(PostProcessInner {
        //     shader_key,
        //     render_pipeline_key,
        // });

        Ok(())
    }
}

struct PostProcessInner {
    pub shader_key: ShaderKey,
    pub render_pipeline_key: RenderPipelineKey,
}

impl PostProcess {

    pub fn push_commands(
        &self,
        ctx: &mut RenderContext,
        texture_view: &web_sys::GpuTextureView,
    ) -> crate::error::Result<()> {
        // Here you would set up the fullscreen quad rendering logic
        // For now, we just log that this function was called
        //tracing::info!("Rendering fullscreen quad with texture view: {:?}", texture_view);

        // render_pass.set_bind_group(0, &texture_bind_group, &[]);
        // render_pass.set_vertex_buffer(0, fullscreen_quad_buffer.slice(..));
        // render_pass.draw(0..6, 0..1); // Two triangles forming a quad
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PostProcessSettings {
    pub enabled: bool,
    pub tonemapping: Option<ToneMapping>
}

impl Default for PostProcessSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            tonemapping: None,
        }
    }
}

impl PostProcessSettings {
    // pub fn shader_cache_key(&self) -> ShaderCacheKey {
    //     ShaderCacheKey::new(Vec::new(), ShaderCacheKeyMaterial::FullScreenQuad)
    // }

    // pub fn material_deps(&self, scene_tex_dep: MaterialTextureDep) -> MaterialDeps {
    //     MaterialDeps::FullScreenQuad(FullScreenQuadMaterialDeps {
    //         scene_tex_dep
    //     })
    // }

    // pub fn pipeline_layout_cache_key(&self, material_bind_group_layout_key: MaterialBindGroupLayoutKey) -> PipelineLayoutCacheKey {
    //     PipelineLayoutCacheKey::new(material_bind_group_layout_key)
    // }

    // pub fn render_pipeline_cache_key(&self, shader_key: ShaderKey, pipeline_layout_key: PipelineLayoutKey) -> RenderPipelineCacheKey {
    //     RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
    // }
}


#[derive(Debug, Clone)]
pub enum ToneMapping {
    ACES,
}