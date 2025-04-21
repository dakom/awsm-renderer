use std::collections::HashSet;

use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::{
    buffers::{
        bind_group::BIND_GROUP_TRANSFORM_BINDING, dynamic::DynamicBufferKind,
        dynamic_fixed::DynamicFixedBuffer,
    },
    AwsmRenderer,
};

use super::{
    error::{AwsmTransformError, Result},
    Transform,
};

new_key_type! {
    pub struct TransformKey;
}

impl AwsmRenderer {
    pub fn update_transforms(&mut self) {
        self.transforms.update_world();
    }
}

const TRANSFORM_INITIAL_CAPACITY: usize = 32; // 32 elements is a good starting point
const TRANSFORM_BYTE_SIZE: usize = 64; // 4x4 matrix of f32 is 64 bytes
const TRANSFORM_BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment

pub struct Transforms {
    locals: SlotMap<TransformKey, Transform>,
    world_matrices: SecondaryMap<TransformKey, glam::Mat4>,
    children: SecondaryMap<TransformKey, Vec<TransformKey>>,
    parents: SecondaryMap<TransformKey, TransformKey>,
    dirties: HashSet<TransformKey>,
    // not every transform here is definitely a skin joint, just in potential
    dirty_skin_joints: HashSet<TransformKey>, 
    gpu_dirty: bool,
    root_node: TransformKey,
    buffer: DynamicFixedBuffer<TransformKey>,
}

impl Transforms {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let buffer = DynamicFixedBuffer::new(
            TRANSFORM_INITIAL_CAPACITY,
            TRANSFORM_BYTE_SIZE,
            TRANSFORM_BYTE_ALIGNMENT,
            DynamicBufferKind::new_uniform(BIND_GROUP_TRANSFORM_BINDING),
            gpu,
            Some("Transforms".to_string()),
        )?;
        let mut locals = SlotMap::with_key();
        let mut world_matrices = SecondaryMap::new();
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
            dirty_skin_joints: HashSet::new(),
            gpu_dirty: true,
            root_node,
            buffer,
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

        self.update_inner(self.root_node, false);

        self.dirties.clear();
    }

    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if self.gpu_dirty {
            self.buffer.write_to_gpu(gpu)?;
            self.gpu_dirty = false;
        }
        Ok(())
    }

    pub fn take_dirty_skin_joints(&mut self) -> HashSet<TransformKey> {
        std::mem::take(&mut self.dirty_skin_joints)
    }

    pub fn bind_group(&self) -> &web_sys::GpuBindGroup {
        self.buffer.bind_group.as_ref().unwrap()
    }

    pub fn bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        self.buffer.bind_group_layout.as_ref().unwrap()
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
    fn update_inner(&mut self, key: TransformKey, dirty_tracker: bool) -> bool {
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
                std::slice::from_raw_parts(values.as_ptr() as *const u8, TRANSFORM_BYTE_SIZE)
            };
            self.buffer.update(key, values_u8);

            self.dirty_skin_joints.insert(key);
        }

        // safety: can't keep a mutable reference to self while it has a borrow of the iterator
        let children = self.children[key].clone();
        for child in children {
            self.update_inner(child, dirty);
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
