use awsm_renderer_core::{bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType}, buffers::BufferBinding, renderer::AwsmRendererWebGpu};

use crate::{bind_groups::AwsmBindGroupError, bind_group_layout::{BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayouts}, camera::CameraBuffer, lights::Lights, transforms::Transforms};

#[derive(Default)]
pub struct GeometryBindGroups {
    pub camera_lights: GeometryBindGroupCameraLights,
    pub transforms: GeometryBindGroupTransforms,
    pub animation: GeometryBindGroupAnimation,
}

#[derive(Default)]
pub struct GeometryBindGroupCameraLights {
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupCameraLights {
    pub fn recreate(&mut self, gpu: &AwsmRendererWebGpu, bind_group_layouts: &mut BindGroupLayouts, camera: &CameraBuffer, lights: &Lights) -> Result<()> {
        let layout_cache_key = self.get_layout_cache_key();
        let layout_key = bind_group_layouts.get_key(gpu, layout_cache_key)?;
        let layout = bind_group_layouts.get(layout_key)?;
        let descriptor = self.get_bind_group_descriptor(camera, lights, &layout);
        let bind_group = gpu.create_bind_group(&descriptor.into());
        self.set_bind_group(bind_group)?;

        Ok(())
    }
    pub fn get_bind_group(&self) -> Result<&web_sys::GpuBindGroup> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry camera_lights".to_string()))
    }

    pub fn get_layout_cache_key(&self) -> BindGroupLayoutCacheKey {
        BindGroupLayoutCacheKey { entries: vec![
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform)),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(BufferBindingLayout::new().with_binding_type(BufferBindingType::ReadOnlyStorage)),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
        ] }
    }

    pub fn get_bind_group_descriptor<'a>(
        &self,
        camera_buffer: &'a CameraBuffer,
        lights: &'a Lights,
        layout: &'a web_sys::GpuBindGroupLayout,
    ) -> BindGroupDescriptor<'a> {

        let entries = vec![
            BindGroupEntry::new(0, BindGroupResource::Buffer(BufferBinding::new(&camera_buffer.gpu_buffer))),
            BindGroupEntry::new(1, BindGroupResource::Buffer(BufferBinding::new(&lights.gpu_buffer))),
        ];

        BindGroupDescriptor::new(layout, Some("Geometry Camera/Lights"), entries)
    }

    pub fn set_bind_group(
        &mut self,
        bind_group: web_sys::GpuBindGroup,
    ) -> Result<()> {
        self._bind_group = Some(bind_group);
        Ok(())
    }
}


#[derive(Default)]
pub struct GeometryBindGroupTransforms {
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupTransforms {
    pub fn recreate(&mut self, gpu: &AwsmRendererWebGpu, bind_group_layouts: &mut BindGroupLayouts, transforms: &Transforms) -> Result<()> {
        // TODO
        Ok(())
    }

    pub fn get_bind_group(&self) -> Result<&web_sys::GpuBindGroup> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry transform".to_string()))
    }
}

#[derive(Default)]
pub struct GeometryBindGroupAnimation {
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl GeometryBindGroupAnimation {
    pub fn recreate(&mut self, gpu: &AwsmRendererWebGpu, bind_group_layouts: &mut BindGroupLayouts) -> Result<()> {
        // TODO
        Ok(())
    }

    pub fn get_bind_group(&self) -> Result<&web_sys::GpuBindGroup> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Geometry animation".to_string()))
    }
}

type Result<T> = std::result::Result<T, AwsmBindGroupError>;