use crate::error::Result;
use crate::render_passes::{
    material::transparent::bind_group::MaterialTransparentBindGroups, RenderPassInitContext,
};

pub struct MaterialTransparentPipelines {}

impl MaterialTransparentPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &MaterialTransparentBindGroups,
    ) -> Result<Self> {
        Ok(Self {})
    }
}
