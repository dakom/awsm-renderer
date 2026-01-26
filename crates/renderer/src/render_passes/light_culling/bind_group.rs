//! Light culling bind group setup.

use crate::bind_groups::{AwsmBindGroupError, BindGroupRecreateContext};
use crate::error::Result;
use crate::render_passes::RenderPassInitContext;

/// Bind group layout and cached bind group for light culling.
pub struct LightCullingBindGroups {
    //pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl LightCullingBindGroups {
    /// Creates bind group layout state for light culling.
    pub async fn new(_ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        Ok(Self {
            //bind_group_layout_key,
            _bind_group: None,
        })
    }

    /// Returns the active light culling bind group.
    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Light Culling".to_string()))
    }

    /// Recreates the light culling bind group.
    pub fn recreate(&mut self, _ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        //self._bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }
}
