use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType,
    },
    buffers::{BufferBinding, BufferDescriptor, BufferUsage},
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use thiserror::Error;

use crate::{camera::CameraBuffer, mesh::morphs::Morphs, skin::Skins, transform::Transforms};

pub struct BindGroups {
    universal: BindGroup,
    mesh_all: BindGroup,
    mesh_shape: BindGroup,
}

struct BindGroup {
    bind_group: web_sys::GpuBindGroup,
    layout: web_sys::GpuBindGroupLayout,
    buffers: Vec<web_sys::GpuBuffer>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BindGroupIndex {
    Universal(UniversalBindGroupBinding),
    MeshAll(MeshAllBindGroupBinding),
    MeshShape(MeshShapeBindGroupBinding),
}

impl BindGroupIndex {
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
}

impl UniversalBindGroupBinding {
    pub fn all() -> [Self; 1] {
        [Self::Camera]
    }

    pub fn initial_buffer_size(self) -> usize {
        match self {
            Self::Camera => CameraBuffer::BYTE_SIZE,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Camera => "Camera",
        }
    }

    pub fn buffer_usage(self) -> BufferUsage {
        match self {
            Self::Camera => BufferUsage::new().with_uniform().with_copy_dst(),
        }
    }

    pub fn bind_group_entry(self, buffer: &web_sys::GpuBuffer) -> BindGroupEntry {
        BindGroupEntry::new(
            self as u32,
            match self {
                Self::Camera => BindGroupResource::Buffer(BufferBinding::new(buffer)),
            },
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MeshAllBindGroupBinding {
    Transform = 0,
}

impl MeshAllBindGroupBinding {
    pub fn all() -> [Self; 1] {
        [Self::Transform]
    }

    pub fn initial_buffer_size(self) -> usize {
        match self {
            Self::Transform => Transforms::INITIAL_CAPACITY * Transforms::BYTE_ALIGNMENT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Transform => "Transform",
        }
    }

    pub fn buffer_usage(self) -> BufferUsage {
        match self {
            Self::Transform => BufferUsage::new().with_uniform().with_copy_dst(),
        }
    }

    pub fn bind_group_entry(self, buffer: &web_sys::GpuBuffer) -> BindGroupEntry {
        BindGroupEntry::new(
            self as u32,
            match self {
                Self::Transform => BindGroupResource::Buffer(
                    BufferBinding::new(buffer).with_size(Transforms::BYTE_ALIGNMENT),
                ),
            },
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MeshShapeBindGroupBinding {
    MorphTargetWeights = 0,
    MorphTargetValues = 1,
    SkinJointMatrices = 2,
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
            Self::MorphTargetWeights => {
                Morphs::WEIGHTS_INITIAL_CAPACITY * Morphs::WEIGHTS_BYTE_ALIGNMENT
            }
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
            Self::MorphTargetWeights => BufferUsage::new().with_uniform().with_copy_dst(),
            Self::MorphTargetValues => BufferUsage::new().with_storage().with_copy_dst(),
            Self::SkinJointMatrices => BufferUsage::new().with_storage().with_copy_dst(),
        }
    }

    pub fn bind_group_entry(self, buffer: &web_sys::GpuBuffer) -> BindGroupEntry {
        BindGroupEntry::new(
            self as u32,
            match self {
                Self::MorphTargetWeights => BindGroupResource::Buffer(
                    BufferBinding::new(buffer).with_size(Morphs::WEIGHTS_BYTE_ALIGNMENT),
                ),
                Self::MorphTargetValues => BindGroupResource::Buffer(BufferBinding::new(buffer)),
                Self::SkinJointMatrices => BindGroupResource::Buffer(BufferBinding::new(buffer)),
            },
        )
    }
}

impl BindGroups {
    pub const UNIVERSAL_INDEX: u32 = 0;
    pub const MESH_ALL_INDEX: u32 = 1;
    pub const MESH_SHAPE_INDEX: u32 = 2;

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let universal = {
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
                vec![BindGroupLayoutEntry::new(
                    UniversalBindGroupBinding::Camera as u32,
                    BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
                    ),
                )
                .with_visibility_vertex()
                .with_visibility_fragment()],
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

            BindGroup {
                bind_group,
                layout,
                buffers,
            }
        };

        let mesh_all = {
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
                vec![BindGroupLayoutEntry::new(
                    MeshAllBindGroupBinding::Transform as u32,
                    BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::Uniform)
                            .with_dynamic_offset(true),
                    ),
                )
                .with_visibility_vertex()],
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

            BindGroup {
                bind_group,
                layout,
                buffers,
            }
        };

        let mesh_shape = {
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
                                .with_binding_type(BufferBindingType::Uniform)
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

            BindGroup {
                bind_group,
                layout,
                buffers,
            }
        };

        Ok(Self {
            universal,
            mesh_all,
            mesh_shape,
        })
    }

    pub fn gpu_write(
        &self,
        gpu: &AwsmRendererWebGpu,
        index: BindGroupIndex,
        buffer_offset: Option<usize>,
        data: &[u8],
        data_offset: Option<usize>,
        data_size: Option<usize>,
    ) -> Result<()> {
        let gpu_buffer = match index {
            BindGroupIndex::Universal(binding) => &self.universal.buffers[binding as u32 as usize],
            BindGroupIndex::MeshAll(binding) => &self.mesh_all.buffers[binding as u32 as usize],
            BindGroupIndex::MeshShape(binding) => &self.mesh_shape.buffers[binding as u32 as usize],
        };
        gpu.write_buffer(gpu_buffer, buffer_offset, data, data_offset, data_size)
            .map_err(|err| AwsmBindGroupError::WriteBuffer {
                label: index.label(),
                err,
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

    pub fn gpu_resize(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        index: BindGroupIndex,
        new_size: usize,
    ) -> Result<()> {
        // we need to recreate the buffer and bind group
        // but *not* the layout
        match index {
            BindGroupIndex::Universal(binding) => {
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
            BindGroupIndex::MeshAll(binding) => {
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
            BindGroupIndex::MeshShape(binding) => {
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

fn gpu_create_buffer(
    gpu: &AwsmRendererWebGpu,
    label: &'static str,
    usage: BufferUsage,
    size: usize,
) -> Result<web_sys::GpuBuffer> {
    gpu.create_buffer(&BufferDescriptor::new(Some(label), size, usage).into())
        .map_err(|err| AwsmBindGroupError::CreateBuffer { label, err })
}

fn gpu_create_layout(
    gpu: &AwsmRendererWebGpu,
    label: &'static str,
    entries: Vec<BindGroupLayoutEntry>,
) -> Result<web_sys::GpuBindGroupLayout> {
    gpu.create_bind_group_layout(
        &BindGroupLayoutDescriptor::new(Some(label))
            .with_entries(entries)
            .into(),
    )
    .map_err(|err| AwsmBindGroupError::Layout {
        bind_group: label,
        err,
    })
}

fn gpu_create_bind_group(
    gpu: &AwsmRendererWebGpu,
    label: &'static str,
    layout: &web_sys::GpuBindGroupLayout,
    entries: Vec<BindGroupEntry>,
) -> web_sys::GpuBindGroup {
    gpu.create_bind_group(&BindGroupDescriptor::new(&layout, Some(label), entries).into())
}

impl Drop for BindGroup {
    fn drop(&mut self) {
        for buffer in &self.buffers {
            buffer.destroy();
        }
    }
}

type Result<T> = std::result::Result<T, AwsmBindGroupError>;

#[derive(Error, Debug)]
pub enum AwsmBindGroupError {
    #[error("[bind group] Error creating buffer for {label}: {err:?}")]
    CreateBuffer {
        label: &'static str,
        err: AwsmCoreError,
    },
    #[error("[bind group] Error creating bind group layout for group {bind_group}: {err:?}")]
    Layout {
        bind_group: &'static str,
        err: AwsmCoreError,
    },

    #[error("[bind group] Error writing buffer for {label}: {err:?}")]
    WriteBuffer {
        label: &'static str,
        err: AwsmCoreError,
    },
}
