#[derive(Default, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderCacheKeyFragmentPostProcess {
    pub gamma_correction: bool,
}
