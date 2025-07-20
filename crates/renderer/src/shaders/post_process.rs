
#[derive(Default, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostProcessShaderCacheKeyMaterial {
    pub gamma_correction: bool,
}

impl From<PostProcessShaderCacheKeyMaterial> for PostProcessShaderTemplateMaterial {
    fn from(key: PostProcessShaderCacheKeyMaterial) -> Self {
        Self {
            gamma_correction: key.gamma_correction,
        }
    }
}

#[derive(Debug)]
pub struct PostProcessShaderTemplateMaterial {
    pub gamma_correction: bool,
}

impl PostProcessShaderTemplateMaterial {
    pub fn new(gamma_correction: bool) -> Self {
        Self { gamma_correction }
    }
}