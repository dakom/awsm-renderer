use crate::render_passes::{light_culling::bind_group::LightCullingBindGroups, RenderPassInitContext};
use crate::error::Result;

pub struct LightCullingPipelines {
}

impl LightCullingPipelines {
    pub async fn new(ctx: &mut RenderPassInitContext, bind_groups: &LightCullingBindGroups) -> Result<Self> {
        Ok(Self {
        })
    }
}