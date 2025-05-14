use awsm_renderer_core::{
    buffers::{BufferDescriptor, BufferUsage},
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use slotmap::SecondaryMap;
use thiserror::Error;

use crate::{
    bind_groups::AwsmBindGroupError,
    buffer::dynamic_storage::DynamicStorageBuffer,
    transform::{Transform, TransformKey, Transforms},
    AwsmRendererLogging,
};

pub struct Instances {
    transform_buffer: DynamicStorageBuffer<TransformKey>,
    transform_count: SecondaryMap<TransformKey, usize>,
    gpu_transform_buffer: web_sys::GpuBuffer,
    transform_gpu_dirty: bool,
}

impl Instances {
    pub const TRANSFORM_INITIAL_SIZE: usize = Transforms::BYTE_SIZE * 32; // 32 elements is a good starting point

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let transform_buffer = DynamicStorageBuffer::new(
            Self::TRANSFORM_INITIAL_SIZE,
            Some("Instance Transforms".to_string()),
        );

        Ok(Self {
            transform_buffer,
            gpu_transform_buffer: gpu_create_vertex_buffer(gpu, Self::TRANSFORM_INITIAL_SIZE)?,
            transform_count: SecondaryMap::new(),
            transform_gpu_dirty: false,
        })
    }

    pub fn transform_insert(&mut self, key: TransformKey, transforms: &[Transform]) {
        let mut bytes = Vec::with_capacity(transforms.len() * Transforms::BYTE_SIZE);
        for transform in transforms {
            let values = transform.to_matrix().to_cols_array();
            let values_u8 = unsafe {
                std::slice::from_raw_parts(values.as_ptr() as *const u8, Transforms::BYTE_SIZE)
            };
            bytes.extend_from_slice(values_u8);
        }

        self.transform_buffer.update(key, &bytes);

        self.transform_count.insert(key, transforms.len());

        self.transform_gpu_dirty = true;
    }

    pub fn transform_update(&mut self, key: TransformKey, index: usize, transform: &Transform) {
        self.transform_buffer.update_with_unchecked(key, |bytes| {
            let offset = index * Transforms::BYTE_SIZE;
            let values = transform.to_matrix().to_cols_array();
            let values_u8 = unsafe {
                std::slice::from_raw_parts(values.as_ptr() as *const u8, Transforms::BYTE_SIZE)
            };

            let slice = &mut bytes[offset..offset + Transforms::BYTE_SIZE];
            slice.copy_from_slice(values_u8);
        });

        self.transform_gpu_dirty = true;
    }

    pub fn transform_buffer_offset(&self, key: TransformKey) -> Result<usize> {
        self.transform_buffer
            .offset(key)
            .ok_or(AwsmInstanceError::TransformNotFound(key))
    }

    pub fn gpu_transform_buffer(&self) -> &web_sys::GpuBuffer {
        &self.gpu_transform_buffer
    }

    pub fn transform_instance_count(&self, key: TransformKey) -> Option<usize> {
        self.transform_count.get(key).copied()
    }

    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
    ) -> Result<()> {
        if self.transform_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Instance Transform GPU write").entered())
            } else {
                None
            };

            if let Some(new_size) = self.transform_buffer.take_gpu_needs_resize() {
                self.gpu_transform_buffer = gpu_create_vertex_buffer(gpu, new_size)?;
            }
            gpu.write_buffer(
                &self.gpu_transform_buffer,
                None,
                self.transform_buffer.raw_slice(),
                None,
                None,
            )?;

            self.transform_gpu_dirty = false;
        }
        Ok(())
    }
}

fn gpu_create_vertex_buffer(gpu: &AwsmRendererWebGpu, size: usize) -> Result<web_sys::GpuBuffer> {
    Ok(gpu.create_buffer(
        &BufferDescriptor::new(
            Some("InstanceTransformVertex"),
            size,
            BufferUsage::new().with_copy_dst().with_vertex(),
        )
        .into(),
    )?)
}

type Result<T> = std::result::Result<T, AwsmInstanceError>;

#[derive(Error, Debug)]
pub enum AwsmInstanceError {
    #[error("[instance] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[instance] {0:?}")]
    WriteBuffer(#[from] AwsmBindGroupError),

    #[error("[instance] transform does not exist {0:?}")]
    TransformNotFound(TransformKey),
}
