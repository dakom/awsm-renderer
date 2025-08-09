use crate::error::Result;
use crate::render_passes::{
    light_culling::bind_group::LightCullingBindGroups, RenderPassInitContext,
};

pub struct LightCullingPipelines {}

impl LightCullingPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &LightCullingBindGroups,
    ) -> Result<Self> {
        Ok(Self {})
    }
}
