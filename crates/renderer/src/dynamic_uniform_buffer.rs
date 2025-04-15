use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType,
    },
    buffer::{BufferBinding, BufferDescriptor, BufferUsage},
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use slotmap::{Key, SecondaryMap};

/// This gives us a generic helper for dynamic uniform buffers.
/// It internally manages free slots for re‑use, and reallocates (grows) the underlying buffer only when needed.
///
/// The bind group layout and bind group are created once (and updated on buffer reallocation)
/// so that even with thousands of draw calls, we only use one bind group layout.
#[derive(Debug)]
pub(super) struct DynamicUniformBuffer<K: Key, const BYTE_SIZE: usize, const ZERO_VALUE: u8 = 0> {
    /// Raw CPU‑side data for all items, organized in BYTE_SIZE slots.
    pub raw_data: Vec<u8>,
    /// The GPU buffer storing the raw data.
    pub gpu_buffer: web_sys::GpuBuffer,
    pub gpu_buffer_needs_resize: bool,
    /// Mapping from a Key to a slot index within the buffer.
    pub slot_indices: SecondaryMap<K, usize>,
    /// The bind group used for binding this buffer in shaders.
    pub bind_group: web_sys::GpuBindGroup,
    /// The bind group layout (static, created once).
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
    /// List of free slot indices available for reuse.
    pub free_slots: Vec<usize>,
    /// Total capacity of the buffer in number of slots.
    pub capacity_slots: usize,
    pub label: Option<String>,
}

impl<K: Key, const BYTE_SIZE: usize, const ZERO_VALUE: u8>
    DynamicUniformBuffer<K, BYTE_SIZE, ZERO_VALUE>
{
    // Just a reasonable default
    const INITIAL_CAPACITY: usize = 32;
    // minUniformBufferOffsetAlignment
    const SLOT_SIZE_ALIGNED: usize = 256;
    const INITIAL_SIZE_BYTES: usize = Self::INITIAL_CAPACITY * Self::SLOT_SIZE_ALIGNED;

    pub fn new(
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> std::result::Result<Self, AwsmCoreError> {
        // Allocate CPU data – initially filled with zeros.
        let raw_data = vec![ZERO_VALUE; Self::INITIAL_SIZE_BYTES];

        // Create the GPU buffer.
        let gpu_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                label.as_deref(),
                Self::INITIAL_SIZE_BYTES,
                BufferUsage::new().with_copy_dst().with_uniform(),
            )
            .into(),
        )?;

        // Create the bind group layout (one binding, marked as dynamic).
        let bind_group_layout = gpu.create_bind_group_layout(
            &BindGroupLayoutDescriptor::new(label.as_deref())
                .with_entries(vec![BindGroupLayoutEntry::new(
                    0,
                    BindGroupLayoutResource::Buffer(
                        BufferBindingLayout::new()
                            .with_binding_type(BufferBindingType::Uniform)
                            .with_dynamic_offset(true)
                            .with_min_binding_size(Self::SLOT_SIZE_ALIGNED),
                    ),
                )
                .with_visibility_vertex()])
                .into(),
        )?;

        let bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &bind_group_layout,
                label.as_deref(),
                vec![BindGroupEntry::new(
                    0,
                    BindGroupResource::Buffer(
                        BufferBinding::new(&gpu_buffer)
                            .with_offset(0)
                            .with_size(Self::SLOT_SIZE_ALIGNED),
                    ),
                )],
            )
            .into(),
        );

        Ok(Self {
            raw_data,
            gpu_buffer,
            gpu_buffer_needs_resize: false,
            slot_indices: SecondaryMap::new(),
            bind_group,
            bind_group_layout,
            free_slots: (0..Self::INITIAL_CAPACITY).collect(),
            capacity_slots: Self::INITIAL_CAPACITY,
            label,
        })
    }

    // Inserts a new item into the buffer.
    // this will efficiently:
    // * write into the slot if it already has one
    // * use a free slot if available
    // * grow the buffer if needed
    // It does not touch the GPU, and can be called many times a frame
    pub fn update(&mut self, key: K, values: &[u8]) {
        // If we don't have a slot, set one
        let slot = match self.slot_indices.get(key) {
            Some(slot) => *slot,
            None => {
                // Choose a slot: either reuse a free slot or use the next available slot.
                let slot = if let Some(free_slot) = self.free_slots.pop() {
                    free_slot
                } else {
                    let new_slot = self.capacity_slots;
                    // Check if we need to grow the raw_data and GPU buffer.
                    if (new_slot + 1) * Self::SLOT_SIZE_ALIGNED > self.raw_data.len() {
                        self.resize(new_slot + 1);
                    }
                    // Increase our logical capacity count.
                    self.capacity_slots += 1;
                    new_slot
                };

                self.slot_indices.insert(key, slot);

                slot
            }
        };

        // Calculate byte offset.
        let offset_bytes = slot * Self::SLOT_SIZE_ALIGNED;

        // Write values into raw_data.
        self.raw_data[offset_bytes..offset_bytes + BYTE_SIZE].copy_from_slice(values);
    }

    pub fn write_to_gpu(
        &mut self,
        gpu: &AwsmRendererWebGpu,
    ) -> std::result::Result<(), AwsmCoreError> {
        if self.gpu_buffer_needs_resize {
            // Create a new GPU buffer with the new size.
            self.gpu_buffer = gpu.create_buffer(
                &BufferDescriptor::new(
                    self.label.as_deref(),
                    self.raw_data.len(),
                    BufferUsage::new().with_copy_dst().with_uniform(),
                )
                .into(),
            )?;

            // Replace the bind group to point at the new buffer
            self.bind_group = gpu.create_bind_group(
                &BindGroupDescriptor::new(
                    &self.bind_group_layout,
                    self.label.as_deref(),
                    vec![BindGroupEntry::new(
                        0,
                        BindGroupResource::Buffer(
                            BufferBinding::new(&self.gpu_buffer)
                                .with_offset(0)
                                .with_size(Self::SLOT_SIZE_ALIGNED),
                        ),
                    )],
                )
                .into(),
            );

            self.gpu_buffer_needs_resize = false;
        }

        Ok(gpu.write_buffer(&self.gpu_buffer, None, self.raw_data.as_slice(), None, None)?)
    }

    /// Removes the slot corresponding to the given key.
    /// The slot is marked as free for reuse.
    pub fn remove(&mut self, key: K) {
        if let Some(slot) = self.slot_indices.remove(key) {
            // Add this slot to the free list.
            self.free_slots.push(slot);

            // Zero out the data in the slot.
            let offset_bytes = slot * Self::SLOT_SIZE_ALIGNED;
            self.raw_data[offset_bytes..offset_bytes + Self::SLOT_SIZE_ALIGNED].fill(ZERO_VALUE);
        }
    }

    pub fn offset(&self, key: K) -> Option<usize> {
        let slot = self.slot_indices.get(key)?;

        Some(slot * Self::SLOT_SIZE_ALIGNED)
    }

    /// Resizes the buffer so that it can store at least `required_slots`.
    /// This method grows the raw_data and creates a new GPU buffer (and updates the bind group).
    fn resize(&mut self, required_slots: usize) {
        // We grow by doubling the capacity of required slots.
        // Take the max of current capacity vs. required_slots to avoid accidental shrinking
        // though this should really never happen
        self.capacity_slots = self.capacity_slots.max(required_slots) * 2;

        // Resize the CPU-side data; new bytes are zeroed out.
        self.raw_data
            .resize(self.capacity_slots * Self::SLOT_SIZE_ALIGNED, ZERO_VALUE);

        // mark this so it will resize before the next gpu write
        self.gpu_buffer_needs_resize = true;
    }
}
