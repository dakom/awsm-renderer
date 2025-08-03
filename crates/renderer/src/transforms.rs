use glam::{Mat4, Quat, Vec3};
use thiserror::Error;

use std::{collections::{HashMap, HashSet}, sync::LazyLock};

use awsm_renderer_core::{buffers::{BufferDescriptor, BufferUsage}, error::AwsmCoreError, renderer::AwsmRendererWebGpu};
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::{
    bind_groups::{AwsmBindGroupError, BindGroupCreate, BindGroups}, buffer::dynamic_uniform::DynamicUniformBuffer, mesh::skins::AwsmSkinError, AwsmRenderer, AwsmRendererLogging
};

impl AwsmRenderer {
    pub fn update_transforms(&mut self) {
        self.transforms.update_world();
        self.meshes.update_world(self.transforms.take_dirty_meshes());
    }
}

pub struct Transforms {
    locals: SlotMap<TransformKey, Transform>,
    world_matrices: SecondaryMap<TransformKey, glam::Mat4>,
    children: SecondaryMap<TransformKey, Vec<TransformKey>>,
    parents: SecondaryMap<TransformKey, TransformKey>,
    // These are the transforms that are dirtied from the outside
    // e.g. may be set multiples times by the user or randomly in the hierarchy 
    dirties: HashSet<TransformKey>,
    // While we calculate the dirties, we can know if meshes need to be updated
    // this is set internally
    // not every transform here is definitely a mesh, just in potential
    dirty_meshes: Vec<TransformKey>,
    gpu_dirty: bool,
    root_node: TransformKey,
    buffer: DynamicUniformBuffer<TransformKey>,
    pub(crate) gpu_buffer: web_sys::GpuBuffer,
}

static BUFFER_USAGE: LazyLock<BufferUsage> = LazyLock::new(|| BufferUsage::new().with_uniform().with_copy_dst());

impl Transforms {
    pub const INITIAL_CAPACITY: usize = 32; // 32 elements is a good starting point
    pub const BYTE_SIZE: usize = 64; // 4x4 matrix of f32 is 64 bytes
    pub const BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
            Some("Transforms"),
            Transforms::INITIAL_CAPACITY * Transforms::BYTE_ALIGNMENT,
            *BUFFER_USAGE,
        ).into())?;

        let buffer = DynamicUniformBuffer::new(
            Self::INITIAL_CAPACITY,
            Self::BYTE_SIZE,
            Self::BYTE_ALIGNMENT,
            Some("Transforms".to_string()),
        );
        let mut locals = SlotMap::with_capacity_and_key(Self::INITIAL_CAPACITY);
        let mut world_matrices = SecondaryMap::with_capacity(Self::INITIAL_CAPACITY);
        let mut children = SecondaryMap::new();

        let root_node = locals.insert(Transform::default());
        world_matrices.insert(root_node, glam::Mat4::IDENTITY);
        children.insert(root_node, Vec::new());

        Ok(Self {
            locals,
            world_matrices,
            children,
            parents: SecondaryMap::new(),
            dirties: HashSet::new(),
            dirty_meshes: Vec::with_capacity(Self::INITIAL_CAPACITY),
            gpu_dirty: true,
            root_node,
            buffer,
            gpu_buffer,
        })
    }

    pub fn insert(&mut self, transform: Transform, parent: Option<TransformKey>) -> TransformKey {
        let world_matrix = transform.to_matrix();

        let key = self.locals.insert(transform);

        self.world_matrices.insert(key, world_matrix);
        self.children.insert(key, Vec::new());
        self.dirties.insert(key);

        self.set_parent(key, parent);

        key
    }

    pub fn remove(&mut self, key: TransformKey) {
        if key == self.root_node {
            return;
        }

        // happens separately so that we can remove the node from the parent's children list
        self.unset_parent(key);

        self.locals.remove(key);
        self.world_matrices.remove(key);
        self.children.remove(key);
        self.dirties.remove(&key);
        self.buffer.remove(key);

        self.gpu_dirty = true;
    }

    // This is the only way to modify the matrices (since it must manage the dirty flags)
    // world transforms are updated by calling update()
    pub fn set_local(&mut self, key: TransformKey, transform: Transform) -> Result<()> {
        if key == self.root_node {
            return Err(AwsmTransformError::CannotModifyRootNode);
        }
        match self.locals.get_mut(key) {
            Some(existing) => {
                *existing = transform;
                self.dirties.insert(key);
                Ok(())
            }
            None => Err(AwsmTransformError::LocalNotFound(key)),
        }
    }

    // if parent is None then the parent is the root node
    pub fn set_parent(&mut self, child: TransformKey, parent: Option<TransformKey>) {
        if child == self.root_node {
            return;
        }

        let parent = parent.unwrap_or(self.root_node);

        if let Some(existing_parent) = self.parents.get(child) {
            if *existing_parent == parent {
                return;
            } else {
                self.unset_parent(child);
            }
        }

        // safe because all transforms have children vec when created
        self.children.get_mut(parent).unwrap().push(child);

        self.parents.insert(child, parent);
    }

    pub fn get_parent(&self, child: TransformKey) -> Result<TransformKey> {
        if child == self.root_node {
            return Err(AwsmTransformError::CannotGetParentOfRootNode);
        }

        self.parents
            .get(child)
            .copied()
            .ok_or(AwsmTransformError::CannotGetParent(child))
    }

    pub fn get_local(&self, key: TransformKey) -> Result<&Transform> {
        self.locals
            .get(key)
            .ok_or(AwsmTransformError::LocalNotFound(key))
    }

    pub fn get_world(&self, key: TransformKey) -> Result<&glam::Mat4> {
        self.world_matrices
            .get(key)
            .ok_or(AwsmTransformError::WorldNotFound(key))
    }

    // This is the only way to update the world matrices
    // it does *not* write to the GPU, so it can be called relatively frequently for physics etc.
    pub(crate) fn update_world(&mut self) {
        self.gpu_dirty = self.gpu_dirty || !self.dirties.is_empty();

        self.update_inner_recursively(self.root_node, false);

        self.dirties.clear();
    }

    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Transform GPU write").entered())
            } else {
                None
            };

            if let Some(new_size) = self.buffer.take_gpu_needs_resize() {
                self.gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
                    Some("Transforms"),
                    new_size,
                    *BUFFER_USAGE,
                ).into())?;

                bind_groups.mark_create(BindGroupCreate::TransformsResize);
            }

            gpu.write_buffer(&self.gpu_buffer, None, self.buffer.raw_slice(), None, None)?;

            self.gpu_dirty = false;
        }
        Ok(())
    }

    pub fn take_dirty_meshes(&mut self) -> HashMap<TransformKey, &Mat4> {
        self.dirty_meshes.drain(..).map(|key| {
            // this for sure exists since we just drained the key
            let world_matrix = self.world_matrices.get(key).unwrap();
            (key, world_matrix)
        })
        .collect()
    }

    pub fn buffer_offset(&self, key: TransformKey) -> Result<usize> {
        self.buffer
            .offset(key)
            .ok_or(AwsmTransformError::TransformBufferSlotMissing(key))
    }

    pub fn world_matrices_ref(&self) -> &SecondaryMap<TransformKey, glam::Mat4> {
        &self.world_matrices
    }

    // internal-only function
    // See: https://gameprogrammingpatterns.com/dirty-flag.html
    // the overall idea is we walk the tree and skip over nodes that are not dirty
    // whenever we encounter a dirty node, we must also mark all of its children dirty
    // finally, for each dirty node, its world transform is its parent's world transform
    // multiplied by its local transform
    // or in other words, it's the local transform, offset by its parent in world space
    //
    // we also update the CPU-side buffer as needed so it will be ready for the GPU
    fn update_inner_recursively(&mut self, key: TransformKey, dirty_tracker: bool) -> bool {
        let dirty = self.dirties.contains(&key) | dirty_tracker;

        if dirty {
            let local_matrix = self.locals[key].to_matrix();

            let world_matrix = match self.parents.get(key) {
                Some(parent) => {
                    let parent_matrix = self.world_matrices[*parent];
                    parent_matrix.mul_mat4(&local_matrix)
                }
                None => local_matrix,
            };

            self.world_matrices[key] = world_matrix;

            let values = world_matrix.to_cols_array();
            let values_u8 = unsafe {
                std::slice::from_raw_parts(values.as_ptr() as *const u8, Self::BYTE_SIZE)
            };
            self.buffer.update(key, values_u8);

            self.dirty_meshes.push(key);
        }

        // safety: can't keep a mutable reference to self while it has a borrow of the iterator
        // TODO: maybe split this function into a pure function that takes the deps?
        let children = self.children[key].clone();
        for child in children {
            self.update_inner_recursively(child, dirty);
        }

        dirty
    }

    // internal-only function - leaves the node dangling
    // after this call, the node should either be immediately removed or reparented
    fn unset_parent(&mut self, child: TransformKey) {
        if let Some(parent) = self.parents.remove(child) {
            if let Some(children) = self.children.get_mut(parent) {
                children.retain(|&c| c != child);
            }
        }
    }
}


#[derive(Clone, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

new_key_type! {
    pub struct TransformKey;
}


pub type Result<T> = std::result::Result<T, AwsmTransformError>;

#[derive(Error, Debug)]
pub enum AwsmTransformError {
    #[error("[transform] local transform does not exist {0:?}")]
    LocalNotFound(TransformKey),

    #[error("[transform] world transform does not exist {0:?}")]
    WorldNotFound(TransformKey),

    #[error("[transform] cannot modify root node")]
    CannotModifyRootNode,

    #[error("[transform] buffer slot missing {0:?}")]
    TransformBufferSlotMissing(TransformKey),

    #[error("[transform] cannot get parent of root node")]
    CannotGetParentOfRootNode,

    #[error("[transform] cannot get parent for {0:?}")]
    CannotGetParent(TransformKey),

    #[error("[transform] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[transform] {0:?}")]
    Skin(#[from] AwsmSkinError),
}