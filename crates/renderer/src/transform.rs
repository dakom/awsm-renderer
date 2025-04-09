use std::collections::HashSet;
use awsm_renderer_core::{bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType}, buffer::{BufferBinding, BufferDescriptor, BufferUsage}, error::AwsmCoreError, renderer::AwsmRendererWebGpu};
use glam::Mat4;
use thiserror::Error;

use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::gltf::buffers::debug_slice_to_f32;

pub struct Transforms {
    gpu: AwsmRendererWebGpu,
    locals: SlotMap<TransformKey, Transform>,
    world_matrices: SecondaryMap<TransformKey, glam::Mat4>,
    children: SecondaryMap<TransformKey, Vec<TransformKey>>,
    parents: SecondaryMap<TransformKey, TransformKey>,
    dirties: HashSet<TransformKey>,
    uniforms_to_write: HashSet<TransformKey>,
    root_node: TransformKey,
    buffer: TransformsBuffer,
}


new_key_type! {
    pub struct TransformKey;
}

impl Transforms {
    pub fn new(gpu: AwsmRendererWebGpu) -> Result<Self> {
        let buffer = TransformsBuffer::new(&gpu)?;
        let mut locals = SlotMap::with_key();
        let mut world_matrices = SecondaryMap::new();
        let mut children = SecondaryMap::new();

        let root_node = locals.insert(Transform::default());
        world_matrices.insert(root_node, glam::Mat4::IDENTITY);
        children.insert(root_node, Vec::new());

        Ok(Self {
            gpu,
            locals,
            world_matrices,
            children,
            parents: SecondaryMap::new(),
            dirties: HashSet::new(),
            uniforms_to_write: HashSet::new(),
            root_node,
            buffer
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
        self.uniforms_to_write.remove(&key);
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

    pub fn write_world(&mut self) -> Result<()> {
        for key in self.uniforms_to_write.drain() {
            let transform = self.world_matrices[key];
            self.buffer.write(key, transform, &self.gpu)?;
        }

        Ok(())
    }

    pub fn bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.buffer.bind_group
    }

    pub fn bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.buffer.bind_group_layout
    }

    pub fn buffer_offset(&self, key: TransformKey) -> Result<usize> {
        let slot = self.buffer.slot_indices.get(key).ok_or(AwsmTransformError::TransformBufferSlotMissing(key))?;
        Ok(slot * TransformsBuffer::SLOT_SIZE_ALIGNED)
    }

    // internal-only function
    // See: https://gameprogrammingpatterns.com/dirty-flag.html
    // the overall idea is we walk the tree and skip over nodes that are not dirty
    // whenever we encounter a dirty node, we must also mark all of its children dirty
    // finally, for each dirty node, its world transform is its parent's world transform
    // multiplied by its local transform
    // or in other words, it's the local transform, offset by its parent in world space
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
            self.uniforms_to_write.insert(key);
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


/// TransformsBuffer manages a dynamic uniform buffer.
///
/// Each transform is 64 bytes (a glam::Mat4). Internally, we manage free slots for re‑use,
/// and we reallocate (grow) the underlying buffer when needed.
///
/// The bind group layout and bind group are created once (and updated on buffer reallocation)
/// so that even with thousands of transforms, we only use one bind group layout.
#[derive(Debug)]
struct TransformsBuffer {
    /// Raw CPU‑side data for all transforms, organized in 64‑byte slots.
    pub raw_data: Vec<u8>,
    /// The GPU buffer storing the raw data.
    pub gpu_buffer: web_sys::GpuBuffer,
    /// Mapping from a TransformKey to a slot index within the buffer.
    pub slot_indices: SecondaryMap<TransformKey, usize>,
    /// The bind group used for binding this buffer in shaders.
    pub bind_group: web_sys::GpuBindGroup,
    /// The bind group layout (static, created once).
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
    /// List of free slot indices available for reuse.
    pub free_slots: Vec<usize>,
    /// Total capacity of the buffer in number of transform slots.
    pub capacity_slots: usize,
}

impl TransformsBuffer {
    const INITIAL_CAPACITY: usize = 32;
    const RESIZE_MAX_CAPACITY: usize = 16;
    const TRANSFORM_BYTE_SIZE: usize = 64; // 4x4 matrix of f32 is 64 bytes
    const SLOT_SIZE_ALIGNED: usize = 256; // 4x4 matrix of f32 is just 64 bytes but we need to align to 256
    const INITIAL_SIZE_BYTES:usize = Self::INITIAL_CAPACITY * Self::SLOT_SIZE_ALIGNED;

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {

        // Allocate CPU data – initially filled with zeros.
        let raw_data = vec![0u8; Self::INITIAL_SIZE_BYTES];

        // Create the GPU buffer.
        let gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
            Some("Transforms"),
            Self::INITIAL_SIZE_BYTES,
            BufferUsage::new()
                .with_copy_dst()
                .with_uniform()
        ).into())?;

        // Create the bind group layout (one binding, marked as dynamic).
        let bind_group_layout = gpu.create_bind_group_layout(&BindGroupLayoutDescriptor::new(Some("Transforms"))
            .with_entries(vec![
                BindGroupLayoutEntry::new(0, BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::Uniform)
                        .with_dynamic_offset(true)
                        .with_min_binding_size(Self::SLOT_SIZE_ALIGNED)
                ))
                .with_visibility_vertex()
            ])
        .into())?;

        let bind_group = gpu.create_bind_group(&BindGroupDescriptor::new(
            &bind_group_layout,
            Some("Transforms"),
            vec![BindGroupEntry::new(0, BindGroupResource::Buffer(
                BufferBinding::new(&gpu_buffer)
                    .with_offset(0)
                    .with_size(Self::SLOT_SIZE_ALIGNED)
            ))]
        ).into());

        Ok(Self {
            raw_data,
            gpu_buffer,
            slot_indices: SecondaryMap::new(),
            bind_group,
            bind_group_layout,
            free_slots: (0..Self::INITIAL_CAPACITY).collect(),
            capacity_slots: Self::INITIAL_CAPACITY,
        })
    }

    /// Inserts a new transform into the buffer.
    /// this will efficiently:
    /// * write into the transform's slot if it already has one
    /// * use a free slot if available
    /// * grow the buffer if needed
    pub fn write(&mut self, key: TransformKey, transform: Mat4, gpu: &AwsmRendererWebGpu) -> Result<()> {
        // If we don't have a slot, set one
        if !self.slot_indices.contains_key(key) {
            // Choose a slot: either reuse a free slot or use the next available slot.
            let slot = if let Some(free_slot) = self.free_slots.pop() {
                free_slot
            } else {
                let new_slot = self.capacity_slots;
                // Check if we need to grow the raw_data and GPU buffer.
                if (new_slot + 1) * Self::SLOT_SIZE_ALIGNED > self.raw_data.len() {
                    self.resize(new_slot + 1, gpu)?;
                }
                // Increase our logical capacity count.
                self.capacity_slots += 1;
                new_slot
            };

            self.slot_indices.insert(key, slot);
        }

        self.inner_write_to_slot(key, transform, gpu)?;


        Ok(())
    }

    /// Removes the transform corresponding to the given key.
    /// The slot is marked as free for reuse.
    pub fn remove(&mut self, key: TransformKey) {
        if let Some(slot) = self.slot_indices.remove(key) {
            // Add this slot to the free list.
            self.free_slots.push(slot);
            // (no need to clear the data here)
        }
    }

    /// Resizes the buffer so that it can store at least `required_slots` transforms.
    /// This method grows the raw_data and creates a new GPU buffer (and updates the bind group).
    fn resize(&mut self, required_slots: usize, gpu: &AwsmRendererWebGpu) -> Result<()> {
        // We grow by doubling the capacity (or ensuring it meets required_slots).
        let new_capacity = (self.capacity_slots.max(required_slots) * 2).max(Self::RESIZE_MAX_CAPACITY);
        let new_size_bytes = new_capacity * Self::SLOT_SIZE_ALIGNED;

        // Resize the CPU-side data; new bytes are filled with zero.
        self.raw_data.resize(new_size_bytes, 0);

        // Create a new GPU buffer with the new size.
        let gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
            Some("Transforms"),
            new_size_bytes,
            BufferUsage::new()
                .with_copy_dst()
                .with_uniform()
        ).into())?;

        // Write the entire raw_data into the new GPU buffer.
        gpu.write_buffer(&gpu_buffer, None, self.raw_data.as_slice(), None, None)?;

        // Replace the bind group to point at the new buffer
        self.bind_group = gpu.create_bind_group(&BindGroupDescriptor::new(
            &self.bind_group_layout,
            Some("Transforms"),
            vec![BindGroupEntry::new(0, BindGroupResource::Buffer(
                BufferBinding::new(&gpu_buffer)
                    .with_offset(0)
                    .with_size(Self::SLOT_SIZE_ALIGNED)
            ))]
        ).into());

        // Replace the old GPU buffer with the new one. 
        self.gpu_buffer = gpu_buffer;

        // Update our capacity.
        self.capacity_slots = new_capacity;

        Ok(())
    }

    /// Updates an existing transform with the given key.
    fn inner_write_to_slot(&mut self, key: TransformKey, transform: Mat4, gpu: &AwsmRendererWebGpu) -> Result<()> {
        let slot = self.slot_indices.get(key).ok_or(AwsmTransformError::TransformBufferSlotMissing(key))?;
        // Calculate byte offset.
        let offset_bytes = slot * Self::SLOT_SIZE_ALIGNED;

        // get the transform as a slice of bytes
        let values = transform.to_cols_array();
        let values_u8 =
            unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, Self::TRANSFORM_BYTE_SIZE) };

        // Write transform data into raw_data.
        self.raw_data[offset_bytes..offset_bytes + Self::TRANSFORM_BYTE_SIZE].copy_from_slice(&values_u8);

        // Update the corresponding part of the GPU buffer.
        gpu.write_buffer(
            &self.gpu_buffer,
            Some(offset_bytes),
            &self.raw_data[offset_bytes..offset_bytes + Self::SLOT_SIZE_ALIGNED],
            None,
            Some(Self::SLOT_SIZE_ALIGNED)
        )?;

        Ok(())
    }

}

impl TransformsBuffer {
}

pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    const IDENTITY: Self = Self {
        translation: glam::Vec3::ZERO,
        rotation: glam::Quat::IDENTITY,
        scale: glam::Vec3::ONE,
    };

    pub fn with_translation(mut self, translation: glam::Vec3) -> Self {
        self.translation = translation;
        self
    }
    pub fn with_rotation(mut self, rotation: glam::Quat) -> Self {
        self.rotation = rotation;
        self
    }
    pub fn with_scale(mut self, scale: glam::Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn from_matrix(matrix: glam::Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
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

    #[error("[transform] {0:?}")]
    Core(#[from] AwsmCoreError),
}
