use crate::render_passes::{geometry::bind_group::GeometryBindGroups, RenderPassInitContext};
use crate::error::Result;

pub struct GeometryPipelines {
}

impl GeometryPipelines {
    pub async fn new(ctx: &mut RenderPassInitContext, bind_groups: &GeometryBindGroups) -> Result<Self> {
        Ok(Self {
        })
    }
}