use awsm_renderer_core::{
    bind_groups::{
        BindGroupEntry, BindGroupLayoutEntry, BindGroupLayoutResource, BindGroupResource,
        BufferBindingLayout, BufferBindingType,
    },
    buffers::{BufferBinding, BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
};

use super::{gpu_create_bind_group, gpu_create_layout, AwsmBindGroupError, Result};
use crate::{
    camera::CameraBuffer, lights::Lights, materials::pbr::PbrMaterial, mesh::morphs::Morphs,
    skin::Skins, transform::Transforms,
};

pub struct UniformStorageBindGroups {
    universal: UniformStorageBindGroup,
    mesh_all: UniformStorageBindGroup,
    mesh_shape: UniformStorageBindGroup,
}

impl UniformStorageBindGroups {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let universal = create_universal_bind_group(gpu)?;
        let mesh_all = create_mesh_all_bind_group(gpu)?;
        let mesh_shape = create_mesh_shape_bind_group(gpu)?;

        Ok(Self {
            universal,
            mesh_all,
            mesh_shape,
        })
    }

    pub fn gpu_universal_bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.universal.bind_group
    }
    pub fn gpu_mesh_all_bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.mesh_all.bind_group
    }
    pub fn gpu_mesh_shape_bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.mesh_shape.bind_group
    }

    pub fn gpu_universal_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.universal.layout
    }
    pub fn gpu_mesh_all_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.mesh_all.layout
    }
    pub fn gpu_mesh_shape_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.mesh_shape.layout
    }

    pub fn gpu_write(
        &self,
        gpu: &AwsmRendererWebGpu,
        index: UniformStorageBindGroupIndex,
        buffer_offset: Option<usize>,
        data: &[u8],
        data_offset: Option<usize>,
        data_size: Option<usize>,
    ) -> Result<()> {
        let gpu_buffer = match index {
            UniformStorageBindGroupIndex::Universal(binding) => {
                &self.universal.buffers[binding as u32 as usize]
            }
            UniformStorageBindGroupIndex::MeshAll(binding) => {
                &self.mesh_all.buffers[binding as u32 as usize]
            }
            UniformStorageBindGroupIndex::MeshShape(binding) => {
                &self.mesh_shape.buffers[binding as u32 as usize]
            }
        };

        gpu.write_buffer(gpu_buffer, buffer_offset, data, data_offset, data_size)
            .map_err(|err| AwsmBindGroupError::WriteBuffer {
                label: index.label(),
                err,
            })
    }

    pub fn gpu_resize(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        index: UniformStorageBindGroupIndex,
        new_size: usize,
    ) -> Result<()> {
        // we need to recreate the buffer and bind group
        // but *not* the layout
        match index {
            UniformStorageBindGroupIndex::Universal(binding) => {
                let buffer =
                    gpu_create_buffer(gpu, binding.label(), binding.buffer_usage(), new_size)?;
                self.universal.buffers[binding as usize] = buffer;
                self.universal.bind_group = gpu_create_bind_group(
                    gpu,
                    "Universal",
                    &self.universal.layout,
                    UniversalBindGroupBinding::all()
                        .into_iter()
                        .map(|binding| {
                            binding.bind_group_entry(&self.universal.buffers[binding as usize])
                        })
                        .collect(),
                );
            }
            UniformStorageBindGroupIndex::MeshAll(binding) => {
                let buffer =
                    gpu_create_buffer(gpu, binding.label(), binding.buffer_usage(), new_size)?;
                self.mesh_all.buffers[binding as usize] = buffer;
                self.mesh_all.bind_group = gpu_create_bind_group(
                    gpu,
                    "MeshAll",
                    &self.mesh_all.layout,
                    MeshAllBindGroupBinding::all()
                        .into_iter()
                        .map(|binding| {
                            binding.bind_group_entry(&self.mesh_all.buffers[binding as usize])
                        })
                        .collect(),
                );
            }
            UniformStorageBindGroupIndex::MeshShape(binding) => {
                let buffer =
                    gpu_create_buffer(gpu, binding.label(), binding.buffer_usage(), new_size)?;
                self.mesh_shape.buffers[binding as usize] = buffer;
                self.mesh_shape.bind_group = gpu_create_bind_group(
                    gpu,
                    "MeshShape",
                    &self.mesh_shape.layout,
                    MeshShapeBindGroupBinding::all()
                        .into_iter()
                        .map(|binding| {
                            binding.bind_group_entry(&self.mesh_shape.buffers[binding as usize])
                        })
                        .collect(),
                );
            }
        }

        Ok(())
    }
}

pub(super) struct UniformStorageBindGroup {
    pub bind_group: web_sys::GpuBindGroup,
    pub layout: web_sys::GpuBindGroupLayout,
    pub buffers: Vec<web_sys::GpuBuffer>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UniformStorageBindGroupIndex {
    Universal(UniversalBindGroupBinding),
    MeshAll(MeshAllBindGroupBinding),
    MeshShape(MeshShapeBindGroupBinding),
}

impl UniformStorageBindGroupIndex {
    pub fn label(self) -> &'static str {
        match self {
            Self::Universal(binding) => binding.label(),
            Self::MeshAll(binding) => binding.label(),
            Self::MeshShape(binding) => binding.label(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum UniversalBindGroupBinding {
    Camera = 0,
    Lights = 1,
}

impl UniversalBindGroupBinding {
    pub fn all() -> [Self; 2] {
        [Self::Camera, Self::Lights]
    }

    pub fn initial_buffer_size(self) -> usize {
        match self {
            Self::Camera => CameraBuffer::BYTE_SIZE,
            Self::Lights => Lights::INITIAL_ELEMENTS * Lights::BYTE_ALIGNMENT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Camera => "Camera",
            Self::Lights => "Lights",
        }
    }

    pub fn buffer_usage(self) -> BufferUsage {
        match self {
            Self::Camera => BufferUsage::new().with_uniform().with_copy_dst(),
            Self::Lights => BufferUsage::new().with_storage().with_copy_dst(),
        }
    }

    pub fn bind_group_entry(self, buffer: &web_sys::GpuBuffer) -> BindGroupEntry {
        BindGroupEntry::new(
            self as u32,
            match self {
                Self::Camera => BindGroupResource::Buffer(BufferBinding::new(buffer)),
                Self::Lights => BindGroupResource::Buffer(BufferBinding::new(buffer)),
            },
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MeshAllBindGroupBinding {
    Transform = 0,
    PbrMaterial = 1,
}

impl MeshAllBindGroupBinding {
    pub fn all() -> [Self; 2] {
        [Self::Transform, Self::PbrMaterial]
    }

    pub fn initial_buffer_size(self) -> usize {
        match self {
            Self::Transform => Transforms::INITIAL_CAPACITY * Transforms::BYTE_ALIGNMENT,
            Self::PbrMaterial => {
                PbrMaterial::INITIAL_ELEMENTS * PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Transform => "Transform",
            Self::PbrMaterial => "PbrMaterial",
        }
    }

    pub fn buffer_usage(self) -> BufferUsage {
        match self {
            Self::Transform => BufferUsage::new().with_uniform().with_copy_dst(),
            Self::PbrMaterial => BufferUsage::new().with_uniform().with_copy_dst(),
        }
    }

    pub fn bind_group_entry(self, buffer: &web_sys::GpuBuffer) -> BindGroupEntry {
        BindGroupEntry::new(
            self as u32,
            match self {
                Self::Transform => BindGroupResource::Buffer(
                    BufferBinding::new(buffer).with_size(Transforms::BYTE_ALIGNMENT),
                ),
                Self::PbrMaterial => BindGroupResource::Buffer(
                    BufferBinding::new(buffer)
                        .with_size(PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT),
                ),
            },
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MeshShapeBindGroupBinding {
    MorphTargetWeights,
    MorphTargetValues,
    SkinJointMatrices,
}

impl MeshShapeBindGroupBinding {
    pub fn all() -> [Self; 3] {
        [
            Self::MorphTargetWeights,
            Self::MorphTargetValues,
            Self::SkinJointMatrices,
        ]
    }

    pub fn initial_buffer_size(self) -> usize {
        match self {
            Self::MorphTargetWeights => Morphs::WEIGHTS_INITIAL_SIZE,
            Self::MorphTargetValues => Morphs::VALUES_INITIAL_SIZE,
            Self::SkinJointMatrices => Skins::SKIN_MATRICES_INITIAL_SIZE,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MorphTargetWeights => "MorphTargetWeights",
            Self::MorphTargetValues => "MorphTargetValues",
            Self::SkinJointMatrices => "SkinJointMatrices",
        }
    }

    pub fn buffer_usage(self) -> BufferUsage {
        match self {
            Self::MorphTargetWeights => BufferUsage::new().with_storage().with_copy_dst(),
            Self::MorphTargetValues => BufferUsage::new().with_storage().with_copy_dst(),
            Self::SkinJointMatrices => BufferUsage::new().with_storage().with_copy_dst(),
        }
    }

    pub fn bind_group_entry(self, buffer: &web_sys::GpuBuffer) -> BindGroupEntry {
        BindGroupEntry::new(
            self as u32,
            match self {
                Self::MorphTargetWeights => BindGroupResource::Buffer(BufferBinding::new(buffer)),
                Self::MorphTargetValues => BindGroupResource::Buffer(BufferBinding::new(buffer)),
                Self::SkinJointMatrices => BindGroupResource::Buffer(BufferBinding::new(buffer)),
            },
        )
    }
}

pub(super) fn create_universal_bind_group(
    gpu: &AwsmRendererWebGpu,
) -> Result<UniformStorageBindGroup> {
    let buffers = UniversalBindGroupBinding::all()
        .into_iter()
        .map(|binding| {
            gpu_create_buffer(
                gpu,
                binding.label(),
                binding.buffer_usage(),
                binding.initial_buffer_size(),
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let layout = gpu_create_layout(
        gpu,
        "Universal",
        vec![
            BindGroupLayoutEntry::new(
                UniversalBindGroupBinding::Camera as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
                ),
            )
            .with_visibility_vertex()
            .with_visibility_fragment(),
            BindGroupLayoutEntry::new(
                UniversalBindGroupBinding::Lights as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
            )
            .with_visibility_vertex()
            .with_visibility_fragment(),
        ],
    )?;

    let bind_group = gpu_create_bind_group(
        gpu,
        "Universal",
        &layout,
        UniversalBindGroupBinding::all()
            .into_iter()
            .map(|binding| binding.bind_group_entry(&buffers[binding as usize]))
            .collect(),
    );

    Ok(UniformStorageBindGroup {
        bind_group,
        layout,
        buffers,
    })
}

pub(super) fn create_mesh_all_bind_group(
    gpu: &AwsmRendererWebGpu,
) -> Result<UniformStorageBindGroup> {
    let buffers = MeshAllBindGroupBinding::all()
        .into_iter()
        .map(|binding| {
            gpu_create_buffer(
                gpu,
                binding.label(),
                binding.buffer_usage(),
                binding.initial_buffer_size(),
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let layout = gpu_create_layout(
        gpu,
        "MeshAll",
        vec![
            BindGroupLayoutEntry::new(
                MeshAllBindGroupBinding::Transform as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::Uniform)
                        .with_dynamic_offset(true),
                ),
            )
            .with_visibility_vertex(),
            BindGroupLayoutEntry::new(
                MeshAllBindGroupBinding::PbrMaterial as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::Uniform)
                        .with_dynamic_offset(true),
                ),
            )
            .with_visibility_fragment(),
        ],
    )?;

    let bind_group = gpu_create_bind_group(
        gpu,
        "MeshAll",
        &layout,
        MeshAllBindGroupBinding::all()
            .into_iter()
            .map(|binding| binding.bind_group_entry(&buffers[binding as usize]))
            .collect(),
    );

    Ok(UniformStorageBindGroup {
        bind_group,
        layout,
        buffers,
    })
}

pub(super) fn create_mesh_shape_bind_group(
    gpu: &AwsmRendererWebGpu,
) -> Result<UniformStorageBindGroup> {
    let buffers = MeshShapeBindGroupBinding::all()
        .into_iter()
        .map(|binding| {
            gpu_create_buffer(
                gpu,
                binding.label(),
                binding.buffer_usage(),
                binding.initial_buffer_size(),
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let layout = gpu_create_layout(
        gpu,
        "MeshShape",
        vec![
            BindGroupLayoutEntry::new(
                MeshShapeBindGroupBinding::MorphTargetWeights as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage)
                        .with_dynamic_offset(true),
                ),
            )
            .with_visibility_vertex(),
            BindGroupLayoutEntry::new(
                MeshShapeBindGroupBinding::MorphTargetValues as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage)
                        .with_dynamic_offset(true),
                ),
            )
            .with_visibility_vertex(),
            BindGroupLayoutEntry::new(
                MeshShapeBindGroupBinding::SkinJointMatrices as u32,
                BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage)
                        .with_dynamic_offset(true),
                ),
            )
            .with_visibility_vertex(),
        ],
    )?;

    let bind_group = gpu_create_bind_group(
        gpu,
        "MeshShape",
        &layout,
        MeshShapeBindGroupBinding::all()
            .into_iter()
            .map(|binding| binding.bind_group_entry(&buffers[binding as usize]))
            .collect(),
    );

    Ok(UniformStorageBindGroup {
        bind_group,
        layout,
        buffers,
    })
}

fn gpu_create_buffer(
    gpu: &AwsmRendererWebGpu,
    label: &'static str,
    usage: BufferUsage,
    size: usize,
) -> Result<web_sys::GpuBuffer> {
    gpu.create_buffer(&BufferDescriptor::new(Some(label), size, usage).into())
        .map_err(|err| AwsmBindGroupError::CreateBuffer { label, err })
}

impl Drop for UniformStorageBindGroup {
    fn drop(&mut self) {
        for buffer in &self.buffers {
            buffer.destroy();
        }
    }
}
