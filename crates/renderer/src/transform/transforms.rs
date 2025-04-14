use std::collections::HashSet;

use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use super::{
    buffer::TransformsBuffer,
    error::{AwsmTransformError, Result},
    Transform,
};

new_key_type! {
    pub struct TransformKey;
}

pub struct Transforms {
    locals: SlotMap<TransformKey, Transform>,
    world_matrices: SecondaryMap<TransformKey, glam::Mat4>,
    children: SecondaryMap<TransformKey, Vec<TransformKey>>,
    parents: SecondaryMap<TransformKey, TransformKey>,
    dirties: HashSet<TransformKey>,
    root_node: TransformKey,
    buffer: TransformsBuffer,
}

impl Transforms {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let buffer = TransformsBuffer::new(gpu)?;
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
        // no need to set uniforms_to_write, that will flow organically from dirty propogation

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
    pub fn update_world(&mut self) -> Result<()> {
        self.update_inner(self.root_node, false);

        self.dirties.clear();

        Ok(())
    }

    // This *does* write to the gpu, should be called only once per frame
    pub fn write_buffer(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        self.buffer.write_to_gpu(gpu)
    }

    pub fn bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.buffer.bind_group
    }

    pub fn bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.buffer.bind_group_layout
    }

    pub fn buffer_offset(&self, key: TransformKey) -> Result<usize> {
        let slot = self
            .buffer
            .slot_indices
            .get(key)
            .ok_or(AwsmTransformError::TransformBufferSlotMissing(key))?;
        Ok(slot * TransformsBuffer::SLOT_SIZE_ALIGNED)
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
            self.buffer.update(key, world_matrix);
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
