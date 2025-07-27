use crate::render::post_process::ToneMapping;

#[derive(Default, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderCacheKeyFragmentPostProcess {
    pub gamma_correction: bool,
    pub tonemapping: Option<ToneMapping>,
    pub anti_aliasing: bool,
}
