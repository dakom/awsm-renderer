use std::{collections::HashMap, sync::LazyLock};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    buffers::{BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
    texture::{TextureSampleType, TextureViewDimension},
};

use crate::materials::{pbr::PbrMaterial, AwsmMaterialError, Materials, Result};
use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey,
    },
    bind_groups::{BindGroupCreate, BindGroups},
    buffer::dynamic_uniform::DynamicUniformBuffer,
    materials::{MaterialAlphaMode, MaterialKey},
    textures::{SamplerKey, TextureKey, Textures},
    AwsmRenderer, AwsmRendererLogging,
};

// copy_src is just for debugging
static BUFFER_USAGE: LazyLock<BufferUsage> = LazyLock::new(|| {
    BufferUsage::new()
        .with_uniform()
        .with_copy_dst()
        .with_storage()
});

pub struct PbrMaterialBuffers {
    uniform_buffer: DynamicUniformBuffer<MaterialKey>,
    uniform_buffer_gpu_dirty: bool,
    pub(crate) gpu_buffer: web_sys::GpuBuffer,
}

impl PbrMaterialBuffers {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                Some("Pbr Materials"),
                PbrMaterial::INITIAL_ELEMENTS * Materials::MAX_SIZE,
                *BUFFER_USAGE,
            )
            .into(),
        )?;

        Ok(Self {
            uniform_buffer: DynamicUniformBuffer::new(
                PbrMaterial::INITIAL_ELEMENTS,
                PbrMaterial::BYTE_SIZE,
                Some(Materials::MAX_SIZE),
                Some("PbrUniformBuffer".to_string()),
            ),
            uniform_buffer_gpu_dirty: false,
            gpu_buffer,
        })
    }

    pub fn buffer_offset(&self, key: MaterialKey) -> Option<usize> {
        self.uniform_buffer.offset(key)
    }

    pub fn update(
        &mut self,
        key: MaterialKey,
        pbr_material: &mut PbrMaterial,
        textures: &Textures,
    ) {
        self.uniform_buffer.update_with(key, |offset, data| {
            pbr_material.uniform_buffer_offset = Some(offset);
            let values = pbr_material.uniform_buffer_data(textures);
            data[..values.len()].copy_from_slice(&values);
        });

        self.uniform_buffer_gpu_dirty = true;
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.uniform_buffer_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "PBR Uniform Buffer GPU write").entered())
            } else {
                None
            };

            if let Some(new_size) = self.uniform_buffer.take_gpu_needs_resize() {
                self.gpu_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(Some("Pbr Material"), new_size, *BUFFER_USAGE).into(),
                )?;

                bind_groups.mark_create(BindGroupCreate::PbrMaterialResize);
            }

            gpu.write_buffer(
                &self.gpu_buffer,
                None,
                self.uniform_buffer.raw_slice(),
                None,
                None,
            )?;

            self.uniform_buffer_gpu_dirty = false;
        }
        Ok(())
    }
}
