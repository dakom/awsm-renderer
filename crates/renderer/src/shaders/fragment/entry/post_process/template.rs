use crate::shaders::fragment::entry::post_process::ShaderCacheKeyFragmentPostProcess;
use askama::Template;

#[derive(Template, Debug)]
#[template(path = "fragment/post_process.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateFragmentPostProcess {
    pub gamma_correction: bool,
}

impl ShaderTemplateFragmentPostProcess {
    pub fn new(cache_key: &ShaderCacheKeyFragmentPostProcess) -> Self {
        Self {
            gamma_correction: cache_key.gamma_correction,
        }
    }
}
