use askama::Template;

use crate::{
    render_passes::effects::shader::cache_key::{BloomPhase, ShaderCacheKeyEffects},
    shaders::{AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateEffects {
    pub bind_groups: ShaderTemplateEffectsBindGroups,
    pub compute: ShaderTemplateEffectsCompute,
}

#[derive(Template, Debug)]
#[template(path = "effects_wgsl/bind_groups.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateEffectsBindGroups {
    pub smaa_anti_alias: bool,
    pub multisampled_geometry: bool,
    pub dof: bool,
    pub ping_pong: bool,
    pub debug: ShaderTemplateEffectsDebug,
}

impl ShaderTemplateEffectsBindGroups {
    pub fn new(cache_key: &ShaderCacheKeyEffects) -> Self {
        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
            multisampled_geometry: cache_key.multisampled_geometry,
            dof: cache_key.dof,
            ping_pong: cache_key.ping_pong,
            debug: ShaderTemplateEffectsDebug::new(),
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "effects_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateEffectsCompute {
    pub smaa_anti_alias: bool,
    pub multisampled_geometry: bool,
    /// Bloom is enabled (any phase other than None)
    pub bloom: bool,
    /// First pass: extract bright pixels from composite
    pub bloom_extract: bool,
    /// Final pass: blend bloom result with original
    pub bloom_blend: bool,
    pub dof: bool,
    pub ping_pong: bool,
    pub debug: ShaderTemplateEffectsDebug,
}

impl ShaderTemplateEffectsCompute {
    pub fn new(cache_key: &ShaderCacheKeyEffects) -> Self {
        let bloom = cache_key.bloom_phase != BloomPhase::None;
        let bloom_extract = cache_key.bloom_phase == BloomPhase::Extract;
        let bloom_blend = cache_key.bloom_phase == BloomPhase::Blend;

        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
            multisampled_geometry: cache_key.multisampled_geometry,
            bloom,
            bloom_extract,
            bloom_blend,
            dof: cache_key.dof,
            ping_pong: cache_key.ping_pong,
            debug: ShaderTemplateEffectsDebug::new(),
        }
    }
}

impl TryFrom<&ShaderCacheKeyEffects> for ShaderTemplateEffects {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyEffects) -> Result<Self> {
        Ok(Self {
            bind_groups: ShaderTemplateEffectsBindGroups::new(value),
            compute: ShaderTemplateEffectsCompute::new(value),
        })
    }
}

impl ShaderTemplateEffects {
    pub fn into_source(self) -> Result<String> {
        let bind_groups_source = self.bind_groups.render()?;
        let compute_source = self.compute.render()?;
        Ok(format!("{}\n{}", bind_groups_source, compute_source))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Effects")
    }
}

#[derive(Default, Debug, Clone)]
pub struct ShaderTemplateEffectsDebug {
    pub smaa_edges: bool,
}

impl ShaderTemplateEffectsDebug {
    pub fn new() -> Self {
        Self { smaa_edges: false }
    }
}
