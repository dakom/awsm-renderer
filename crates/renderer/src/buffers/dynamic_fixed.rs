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

//-------------------------------- PERFORMANCE SUMMARY ------------------------//
//
// • insert/update/remove:   O(1)  (amortized, ignoring rare growth)
// • GPU write (per frame):  uploads entire buffer each time
// • Resize strategy:        doubles capacity when needed (infrequent pauses)
// • External fragmentation: none (fixed-size slots)
// • Internal fragmentation: none
// • Memory overhead:        exactly `capacity × ALIGNED_SLICE_SIZE`
//
// • Ideal usage:
//    Thousands of items with identical sizes, like:
//      - Transforms
//      - Morph weights
//      - Skin matrices
//
//----------------------------------------------------------------------------//

/// This gives us a generic helper for dynamic buffers of a fixed alignment size
/// It internally manages free slots for re‑use, and reallocates (grows) the underlying buffer only when needed.
///
/// The bind group layout and bind group are created once (and updated on buffer reallocation)
/// so that even with thousands of draw calls, we only use one bind group layout.
///
/// This is particularly useful for things like transforms and morph weights which have a fixed size,
/// but may be inserted/removed at any time, so we can re-use their slots
/// without having to reallocate the entire buffer every time.
///
/// This also has the benefit of not needing complicated logic to avoid coalescing etc.
#[derive(Debug)]
pub struct DynamicFixedBuffer<K: Key, const ZERO_VALUE: u8 = 0> {
    /// Raw CPU‑side data for all items, organized in BYTE_SIZE slots.
    raw_data: Vec<u8>,
    /// The GPU buffer storing the raw data.
    gpu_buffer: web_sys::GpuBuffer,
    gpu_buffer_needs_resize: bool,
    /// Mapping from a Key to a slot index within the buffer.
    slot_indices: SecondaryMap<K, usize>,
    /// The bind group used for binding this buffer in shaders.
    pub bind_group: web_sys::GpuBindGroup,
    /// The bind group layout (static, created once).
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
    /// List of free slot indices available for reuse.
    free_slots: Vec<usize>,
    /// Total capacity of the buffer in number of slots.
    capacity_slots: usize,
    // first unused index >= capacity used so far
    next_slot: usize,
    label: Option<String>,
    byte_size: usize,
    bind_group_binding: u32,
    aligned_slice_size: usize,
    usage: BufferUsage,
}

impl<K: Key, const ZERO_VALUE: u8> DynamicFixedBuffer<K, ZERO_VALUE> {
    pub fn new_uniform(
        initial_capacity: usize,
        byte_size: usize,
        aligned_slice_size: usize,
        bind_group_binding: u32,
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> std::result::Result<Self, AwsmCoreError> {
        Self::new(
            initial_capacity,
            byte_size,
            aligned_slice_size,
            bind_group_binding,
            BufferBindingType::Uniform,
            BufferUsage::new().with_copy_dst().with_uniform(),
            true,
            false,
            false,
            gpu,
            label,
        )
    }

    pub fn new_storage(
        initial_capacity: usize,
        byte_size: usize,
        aligned_slice_size: usize,
        bind_group_binding: u32,
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> std::result::Result<Self, AwsmCoreError> {
        Self::new(
            initial_capacity,
            byte_size,
            aligned_slice_size,
            bind_group_binding,
            BufferBindingType::ReadOnlyStorage,
            BufferUsage::new().with_copy_dst().with_storage(),
            true,
            false,
            false,
            gpu,
            label,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        initial_capacity: usize,
        byte_size: usize,
        aligned_slice_size: usize,
        bind_group_binding: u32,
        binding_type: BufferBindingType,
        usage: BufferUsage,
        visibility_vertex: bool,
        visibility_fragment: bool,
        visibility_compute: bool,
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> std::result::Result<Self, AwsmCoreError> {
        let initial_size_bytes: usize = initial_capacity * aligned_slice_size;
        // Allocate CPU data – initially filled with zeros.
        let raw_data = vec![ZERO_VALUE; initial_size_bytes];

        // Create the GPU buffer.
        let gpu_buffer = gpu.create_buffer(
            &BufferDescriptor::new(label.as_deref(), initial_size_bytes, usage).into(),
        )?;

        // Create the bind group layout (one binding, marked as dynamic).

        let mut layout_entry = BindGroupLayoutEntry::new(
            bind_group_binding,
            BindGroupLayoutResource::Buffer(
                BufferBindingLayout::new()
                    .with_binding_type(binding_type)
                    .with_dynamic_offset(true),
            ),
        );

        if visibility_vertex {
            layout_entry = layout_entry.with_visibility_vertex();
        }
        if visibility_fragment {
            layout_entry = layout_entry.with_visibility_fragment();
        }
        if visibility_compute {
            layout_entry = layout_entry.with_visibility_compute();
        }

        let bind_group_layout = gpu.create_bind_group_layout(
            &BindGroupLayoutDescriptor::new(label.as_deref())
                .with_entries(vec![layout_entry])
                .into(),
        )?;

        let bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &bind_group_layout,
                label.as_deref(),
                vec![BindGroupEntry::new(
                    bind_group_binding,
                    BindGroupResource::Buffer(
                        BufferBinding::new(&gpu_buffer)
                            // we know exactly how much is used per draw call
                            // so let's just expose that slice
                            .with_size(aligned_slice_size),
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
            free_slots: (0..initial_capacity).collect(),
            capacity_slots: initial_capacity,
            next_slot: initial_capacity,
            label,
            byte_size,
            bind_group_binding,
            aligned_slice_size,
            usage,
        })
    }

    // Inserts or updates an item in the buffer.
    // the values should be of size `byte_size` not `alignment_size`
    //
    // this will efficiently:
    // * write into the slot if it already has one
    // * use a free slot if available
    // * grow the buffer if needed
    // It does not touch the GPU, and can be called many times a frame
    pub fn update_with(&mut self, key: K, f: impl FnOnce(&mut [u8])) {
        // If we don't have a slot, set one
        let slot = match self.slot_indices.get(key) {
            Some(slot) => *slot,
            None => {
                // Choose a slot: either reuse a free slot or use the next available slot.
                let slot = if let Some(free_slot) = self.free_slots.pop() {
                    free_slot
                } else {
                    let new_slot = self.next_slot;
                    // Check if we need to grow the raw_data and GPU buffer.
                    if (new_slot + 1) * self.aligned_slice_size > self.raw_data.len() {
                        self.resize(new_slot + 1);
                    }
                    self.next_slot += 1;
                    new_slot
                };

                self.slot_indices.insert(key, slot);

                slot
            }
        };

        // Calculate byte offset.
        let offset_bytes = slot * self.aligned_slice_size;

        // we can mutate the slice directly
        f(&mut self.raw_data[offset_bytes..offset_bytes + self.byte_size]);
    }

    pub fn update(&mut self, key: K, values: &[u8]) {
        self.update_with(key, |data| {
            data.copy_from_slice(values);
        });
    }

    pub fn write_to_gpu(
        &mut self,
        gpu: &AwsmRendererWebGpu,
    ) -> std::result::Result<(), AwsmCoreError> {
        if self.gpu_buffer_needs_resize {
            // Create a new GPU buffer with the new size.
            self.gpu_buffer = gpu.create_buffer(
                &BufferDescriptor::new(self.label.as_deref(), self.raw_data.len(), self.usage)
                    .into(),
            )?;

            // Replace the bind group to point at the new buffer
            self.bind_group = gpu.create_bind_group(
                &BindGroupDescriptor::new(
                    &self.bind_group_layout,
                    self.label.as_deref(),
                    vec![BindGroupEntry::new(
                        self.bind_group_binding,
                        BindGroupResource::Buffer(
                            BufferBinding::new(&self.gpu_buffer)
                                .with_offset(0)
                                .with_size(self.aligned_slice_size),
                        ),
                    )],
                )
                .into(),
            );

            self.gpu_buffer_needs_resize = false;
        }

        // just write the whole thing :)
        gpu.write_buffer(&self.gpu_buffer, None, self.raw_data.as_slice(), None, None)
    }

    /// Removes the slot corresponding to the given key.
    /// The slot is marked as free for reuse.
    pub fn remove(&mut self, key: K) {
        if let Some(slot) = self.slot_indices.remove(key) {
            // Add this slot to the free list.
            self.free_slots.push(slot);

            // Zero out the data in the slot.
            let offset_bytes = slot * self.aligned_slice_size;
            self.raw_data[offset_bytes..offset_bytes + self.aligned_slice_size].fill(ZERO_VALUE);
        }
    }

    pub fn offset(&self, key: K) -> Option<usize> {
        let slot = self.slot_indices.get(key)?;

        Some(slot * self.aligned_slice_size)
    }

    pub fn keys(&self) -> slotmap::secondary::Keys<K, usize> {
        self.slot_indices.keys()
    }

    fn resize(&mut self, required_slots: usize) {
        // grow to at least double, exactly like Vec
        let new_cap = required_slots.max(self.capacity_slots) * 2;

        self.raw_data
            .resize(new_cap * self.aligned_slice_size, ZERO_VALUE);

        // **avoid duplicating the soon‑to‑be‑allocated slot**
        self.free_slots.extend(required_slots..new_cap);

        self.capacity_slots = new_cap;
        self.gpu_buffer_needs_resize = true;
    }
}
