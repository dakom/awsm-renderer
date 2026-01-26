//! Geometry pass bind group setup.

use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
        BufferBindingLayout, BufferBindingType,
    },
    buffers::BufferBinding,
};

use crate::error::Result;
use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey,
    },
    bind_groups::{AwsmBindGroupError, BindGroupRecreateContext},
    mesh::meta::geometry_meta::GEOMETRY_MESH_META_BYTE_ALIGNMENT,
    render_passes::RenderPassInitContext,
};

/// Bind groups used by the geometry pass.
pub struct GeometryBindGroups {
    pub camera: GeometryBindGroupCamera,
    // these could be be used for multiple meshes
    pub transforms: GeometryBindGroupTransforms,
    // These are more specific to the mesh
    pub meta: GeometryBindGroupMeta,
    pub animation: GeometryBindGroupAnimation,
}

impl GeometryBindGroups {
    /// Creates all geometry bind group layouts.
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let camera = GeometryBindGroupCamera::new(ctx).await?;
        let transforms = GeometryBindGroupTransforms::new(ctx).await?;
        let meta = GeometryBindGroupMeta::new(ctx).await?;
        let animation = GeometryBindGroupAnimation::new(ctx).await?;

        Ok(Self {
            camera,
            transforms,
            meta,
            animation,
        })
    }
}

/// Bind group for camera data in the geometry pass.
pub struct GeometryBindGroupCamera {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupCamera {
    /// Creates the camera bind group layout.
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            }],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    /// Recreates the camera bind group.
    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry Camera"),
            vec![BindGroupEntry::new(
                0,
                BindGroupResource::Buffer(BufferBinding::new(&ctx.camera.gpu_buffer)),
            )],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    /// Returns the active camera bind group.
    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry camera".to_string()))
    }
}

/// Bind group for transform buffers in the geometry pass.
#[derive(Default)]
pub struct GeometryBindGroupTransforms {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupTransforms {
    /// Creates the transforms bind group layout.
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                // Transform
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
            ],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    /// Recreates the transforms bind group.
    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry Transforms"),
            vec![BindGroupEntry::new(
                0,
                BindGroupResource::Buffer(BufferBinding::new(&ctx.transforms.gpu_buffer)),
            )],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    /// Returns the active transforms bind group.
    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry transform".to_string()))
    }
}

/// Bind group for mesh metadata in the geometry pass.
#[derive(Default)]
pub struct GeometryBindGroupMeta {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupMeta {
    /// Creates the metadata bind group layout.
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::Uniform)
                        .with_dynamic_offset(true),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            }],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    /// Recreates the metadata bind group.
    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry meta"),
            vec![BindGroupEntry::new(
                0,
                BindGroupResource::Buffer(
                    BufferBinding::new(ctx.meshes.meta.geometry_gpu_buffer())
                        .with_size(GEOMETRY_MESH_META_BYTE_ALIGNMENT),
                ),
            )],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    /// Returns the active metadata bind group.
    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry meta".to_string()))
    }
}

/// Bind group for morph and skin buffers in the geometry pass.
#[derive(Default)]
pub struct GeometryBindGroupAnimation {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupAnimation {
    /// Creates the animation bind group layout.
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
            ],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    /// Recreates the animation bind group.
    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry animation"),
            vec![
                BindGroupEntry::new(
                    0,
                    BindGroupResource::Buffer(BufferBinding::new(
                        &ctx.meshes.morphs.geometry.gpu_buffer_weights,
                    )),
                ),
                BindGroupEntry::new(
                    1,
                    BindGroupResource::Buffer(BufferBinding::new(
                        &ctx.meshes.morphs.geometry.gpu_buffer_values,
                    )),
                ),
                BindGroupEntry::new(
                    2,
                    BindGroupResource::Buffer(BufferBinding::new(
                        &ctx.meshes.skins.matrices_gpu_buffer,
                    )),
                ),
                BindGroupEntry::new(
                    3,
                    BindGroupResource::Buffer(BufferBinding::new(
                        &ctx.meshes.skins.joint_index_weights_gpu_buffer,
                    )),
                ),
            ],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    /// Returns the active animation bind group.
    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry skin".to_string()))
    }
}
