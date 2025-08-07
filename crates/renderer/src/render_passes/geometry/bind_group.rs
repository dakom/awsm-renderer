use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindGroupLayoutResource,
        BindGroupResource, BufferBindingLayout, BufferBindingType,
    },
    buffers::BufferBinding,
    renderer::AwsmRendererWebGpu,
};

use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey, BindGroupLayouts,
    },
    bind_groups::{AwsmBindGroupError, BindGroupRecreateContext},
    camera::CameraBuffer,
    lights::Lights,
    materials::pbr::PbrMaterial,
    render_passes::{composite::bind_group, RenderPassInitContext},
    transforms::Transforms,
};
use crate::{error::Result, materials::MaterialBufferKind};

pub struct GeometryBindGroups {
    pub camera_lights: GeometryBindGroupCameraLights,
    pub transform_materials: GeometryBindGroupTransformMaterials,
    pub vertex_animation: GeometryBindGroupVertexAnimation,
}

impl GeometryBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        let camera_lights = GeometryBindGroupCameraLights::new(ctx).await?;
        let transform_materials = GeometryBindGroupTransformMaterials::new(ctx).await?;
        let vertex_animation = GeometryBindGroupVertexAnimation::new(ctx).await?;

        Ok(Self {
            camera_lights,
            transform_materials,
            vertex_animation,
        })
    }
}

pub struct GeometryBindGroupCameraLights {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupCameraLights {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
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
            .get_key(&ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry Camera/Lights"),
            vec![
                BindGroupEntry::new(
                    0,
                    BindGroupResource::Buffer(BufferBinding::new(&ctx.camera.gpu_buffer)),
                ),
                BindGroupEntry::new(
                    1,
                    BindGroupResource::Buffer(BufferBinding::new(&ctx.lights.gpu_buffer)),
                ),
            ],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry camera_lights".to_string()))
    }
}

#[derive(Default)]
pub struct GeometryBindGroupTransformMaterials {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupTransformMaterials {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                // Transform
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::Uniform)
                            .with_dynamic_offset(true),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::Uniform)
                            .with_dynamic_offset(true),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
            ],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry Transforms (and materials)"),
            vec![
                BindGroupEntry::new(
                    0,
                    BindGroupResource::Buffer(
                        BufferBinding::new(&ctx.transforms.gpu_buffer)
                            .with_size(Transforms::BYTE_ALIGNMENT),
                    ),
                ),
                BindGroupEntry::new(
                    1,
                    BindGroupResource::Buffer(
                        BufferBinding::new(&ctx.materials.gpu_buffer(MaterialBufferKind::Pbr))
                            .with_size(PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT),
                    ),
                ),
            ],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry transform".to_string()))
    }
}

#[derive(Default)]
pub struct GeometryBindGroupVertexAnimation {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupVertexAnimation {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage)
                            .with_dynamic_offset(true),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage)
                            .with_dynamic_offset(true),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::ReadOnlyStorage)
                            .with_dynamic_offset(true),
                    ),
                    visibility_vertex: true,
                    visibility_fragment: true,
                    visibility_compute: false,
                },
            ],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Geometry vertex animation"),
            vec![
                BindGroupEntry::new(
                    0,
                    BindGroupResource::Buffer(BufferBinding::new(
                        &ctx.meshes.morphs.gpu_buffer_weights,
                    )),
                ),
                BindGroupEntry::new(
                    1,
                    BindGroupResource::Buffer(BufferBinding::new(
                        &ctx.meshes.morphs.gpu_buffer_values,
                    )),
                ),
                BindGroupEntry::new(
                    2,
                    BindGroupResource::Buffer(BufferBinding::new(&ctx.meshes.skins.gpu_buffer)),
                ),
            ],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry vertex animation".to_string()))
    }
}
