//! Light culling pipeline setup.

use crate::error::Result;
use crate::render_passes::{
    light_culling::bind_group::LightCullingBindGroups, RenderPassInitContext,
};

/// Pipeline state for light culling.
pub struct LightCullingPipelines {}

impl LightCullingPipelines {
    /// Creates light culling pipeline state.
    pub async fn new(
        _ctx: &mut RenderPassInitContext<'_>,
        _bind_groups: &LightCullingBindGroups,
    ) -> Result<Self> {
        Ok(Self {})
    }
}
