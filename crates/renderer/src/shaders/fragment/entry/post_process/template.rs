use crate::{
    render::post_process::ToneMapping,
    shaders::fragment::entry::post_process::ShaderCacheKeyFragmentPostProcess,
};
use askama::Template;

#[derive(Template, Debug)]
#[template(path = "fragment/post_process.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateFragmentPostProcess {
    pub gamma_correction: bool,
    pub tonemapping: Option<ToneMapping>,
    pub anti_aliasing: bool,
}

impl ShaderTemplateFragmentPostProcess {
    pub fn new(cache_key: &ShaderCacheKeyFragmentPostProcess) -> Self {
        Self {
            gamma_correction: cache_key.gamma_correction,
            tonemapping: cache_key.tonemapping,
            anti_aliasing: cache_key.anti_aliasing,
        }
    }
}
