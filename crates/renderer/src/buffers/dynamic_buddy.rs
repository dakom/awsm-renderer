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

/// Minimum alloc unit – choose 256 B so every buddy is WebGPU‑aligned.
/// Must be power‑of‑two.
const MIN_BLOCK: usize = 256;

/// This uses "buddy memory allocation" to allow allocations of an arbitrary size
/// that are power‑of‑two aligned. It is a bit more complex and wasteful than the
/// `DynamicFixedBuffer`, but it allows for more flexible allocation sizes
/// with still-excellent performance tradeoffs due to the buddy tree structure.
#[derive(Debug)]
pub struct DynamicBuddyBuffer<K: Key, const ZERO: u8 = 0> {
    raw_data: Vec<u8>,
    /// Complete binary tree stored as an array where each node
    /// is the size of the *largest* free block in that subtree.
    buddy_tree: Vec<usize>,
    slot_indices: SecondaryMap<K, (usize /*offset*/, usize /*size*/)>,

    // --- GPU side & misc ---
    gpu_buffer: web_sys::GpuBuffer,
    gpu_needs_resize: bool,
    pub bind_group: web_sys::GpuBindGroup,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
    usage: BufferUsage,
    bind_group_binding: u32,
    label: Option<String>,
}

impl<K: Key, const ZERO: u8> DynamicBuddyBuffer<K, ZERO> {
    pub fn new_uniform(
        initial_bytes: usize,
        bind: u32,
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> Result<Self, AwsmCoreError> {
        Self::new(
            initial_bytes,
            bind,
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
        initial_bytes: usize,
        bind: u32,
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> Result<Self, AwsmCoreError> {
        Self::new(
            initial_bytes,
            bind,
            BufferBindingType::ReadOnlyStorage,
            BufferUsage::new().with_copy_dst().with_storage(),
            true,
            false,
            false,
            gpu,
            label,
        )
    }

    fn new(
        mut initial_bytes: usize,
        bind: u32,
        binding_type: BufferBindingType,
        usage: BufferUsage,
        visibility_vertex: bool,
        visibility_fragment: bool,
        visibility_compute: bool,
        gpu: &AwsmRendererWebGpu,
        label: Option<String>,
    ) -> Result<Self, AwsmCoreError> {
        // round up to next power‑of‑two multiple of MIN_BLOCK
        initial_bytes = Self::round_pow2(initial_bytes.max(MIN_BLOCK));

        let raw_data = vec![ZERO; initial_bytes];

        let gpu_buffer = gpu
            .create_buffer(&BufferDescriptor::new(label.as_deref(), initial_bytes, usage).into())?;

        // one binding layout
        let mut layout_entry = BindGroupLayoutEntry::new(
            bind,
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
                    bind,
                    // bind the entire buffer (no aligned slice sizes here)
                    BindGroupResource::Buffer(BufferBinding::new(&gpu_buffer)),
                )],
            )
            .into(),
        );

        // buddy tree size: 2 * cap / MIN_BLOCK  – 1  (perfect binary tree)
        let leaves = initial_bytes / MIN_BLOCK;
        let mut buddy_tree = vec![0; 2 * leaves - 1];

        init_full(&mut buddy_tree, 0, initial_bytes); // ← new line

        Ok(Self {
            raw_data,
            buddy_tree,
            slot_indices: SecondaryMap::new(),
            gpu_buffer,
            gpu_needs_resize: false,
            bind_group,
            bind_group_layout,
            usage,
            bind_group_binding: bind,
            label,
        })
    }

    /* ------------------------------------------------------------------ */
    /*                    PUBLIC API: update / remove                     */
    /* ------------------------------------------------------------------ */

    // this is used to both update and insert new data
    // it can be called many times a frame, gpu is only updated with explicit write_to_gpu() call
    pub fn update(&mut self, key: K, bytes: &[u8]) {
        // remove & reinsert if new size doesn’t fit existing block
        if let Some((off, old_size)) = self.slot_indices.get(key).copied() {
            if bytes.len() <= old_size {
                self.raw_data[off..off + bytes.len()].copy_from_slice(bytes);
                // clear unused tail
                if bytes.len() < old_size {
                    self.raw_data[off + bytes.len()..off + old_size].fill(ZERO);
                }
                return;
            }
            self.remove(key);
        }
        self.insert(key, bytes);
    }

    fn insert(&mut self, key: K, bytes: &[u8]) {
        let req = Self::round_pow2(bytes.len().max(MIN_BLOCK));
        let off = self.alloc(req).unwrap_or_else(|| {
            // grow buffer & tree, then retry
            self.grow(req.max(self.raw_data.len()));
            self.alloc(req).expect("allocation after grow must succeed")
        });
        self.raw_data[off..off + bytes.len()].copy_from_slice(bytes);
        self.slot_indices.insert(key, (off, req));
    }

    pub fn remove(&mut self, key: K) {
        if let Some((off, size)) = self.slot_indices.remove(key) {
            self.raw_data[off..off + size].fill(ZERO);
            self.free(off, size);
        }
    }

    // just for debugging
    pub fn used_size(&self) -> usize {
        self.slot_indices
            .values()
            .map(|(_, size)| *size)
            .sum::<usize>()
    }

    /* ------------------------------------------------------------------ */
    /*                GPU write                                           */
    /* ------------------------------------------------------------------ */

    pub fn write_to_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<(), AwsmCoreError> {
        if self.gpu_needs_resize {
            self.gpu_buffer = gpu.create_buffer(
                &BufferDescriptor::new(self.label.as_deref(), self.raw_data.len(), self.usage)
                    .into(),
            )?;
            self.bind_group = gpu.create_bind_group(
                &BindGroupDescriptor::new(
                    &self.bind_group_layout,
                    self.label.as_deref(),
                    vec![BindGroupEntry::new(
                        self.bind_group_binding,
                        BindGroupResource::Buffer(BufferBinding::new(&self.gpu_buffer)),
                    )],
                )
                .into(),
            );
            self.gpu_needs_resize = false;
        }

        //tracing::info!("number of entries: {}, raw size: {}, used size: {}", self.slot_indices.len(), self.raw_data.len(), self.used_size());

        // write the entire buffer
        gpu.write_buffer(&self.gpu_buffer, None, self.raw_data.as_slice(), None, None)
    }

    /* ------------------------------------------------------------------ */
    /*                  Buddy tree helpers                                */
    /* ------------------------------------------------------------------ */

    /// Allocate a block of exactly `req` bytes (power‑of‑two, ≥ MIN_BLOCK).
    fn alloc(&mut self, req: usize) -> Option<usize> {
        let leaves = self.raw_data.len() / MIN_BLOCK;
        let mut idx = 0usize; // start at root
        let size = self.buddy_tree[idx];

        if req > size {
            return None;
        }

        // descend until leaf
        while idx < leaves - 1 {
            let left = idx * 2 + 1;
            if self.buddy_tree[left] >= req {
                idx = left;
            } else {
                idx = left + 1; // go to right child
            }
        }

        // mark this leaf as used
        self.buddy_tree[idx] = 0;
        let mut parent = (idx - 1) >> 1;
        while self.buddy_tree[parent]
            != self.buddy_tree[parent * 2 + 1].max(self.buddy_tree[parent * 2 + 2])
        {
            self.buddy_tree[parent] =
                self.buddy_tree[parent * 2 + 1].max(self.buddy_tree[parent * 2 + 2]);
            if parent == 0 {
                break;
            }
            parent = (parent - 1) >> 1;
        }

        Some(Self::index_to_offset(idx, leaves))
    }

    fn free(&mut self, offset: usize, size: usize) {
        let leaves = self.raw_data.len() / MIN_BLOCK;
        let mut idx = Self::offset_to_index(offset, leaves);
        self.buddy_tree[idx] = size;

        while idx != 0 {
            let parent = (idx - 1) >> 1;
            let left = parent * 2 + 1;
            let right = left + 1;
            let old = self.buddy_tree[parent];
            self.buddy_tree[parent] = self.buddy_tree[left].max(self.buddy_tree[right]);
            if self.buddy_tree[parent] == old {
                break;
            }
            idx = parent;
        }
    }

    fn grow(&mut self, min_extra: usize) {
        let old_cap = self.raw_data.len();
        let mut new_cap = old_cap * 2;
        while new_cap - old_cap < min_extra {
            new_cap *= 2;
        }
        self.raw_data.resize(new_cap, ZERO);
        self.gpu_needs_resize = true;

        // rebuild a new perfect tree
        let leaves = new_cap / MIN_BLOCK;
        self.buddy_tree.clear();
        self.buddy_tree.resize(2 * leaves - 1, 0);
        init_full(&mut self.buddy_tree, 0, new_cap);

        // re‑insert existing allocations so they become *used* leaves
        for (off, _) in self.slot_indices.values() {
            // Marks an existing region as used (when rebuilding tree).
            let leaves = self.raw_data.len() / MIN_BLOCK;
            let mut idx = Self::offset_to_index(*off, leaves);
            self.buddy_tree[idx] = 0;
            while idx != 0 {
                idx = (idx - 1) >> 1;
                let left = idx * 2 + 1;
                let right = left + 1;
                self.buddy_tree[idx] = self.buddy_tree[left].max(self.buddy_tree[right]);
            }
        }
    }

    /* ---- index/offset helpers & math utils ---------------------------- */

    #[inline]
    fn round_pow2(n: usize) -> usize {
        n.next_power_of_two().max(MIN_BLOCK)
    }
    #[inline]
    fn index_to_offset(idx: usize, leaves: usize) -> usize {
        let leaf_idx = idx + 1 - leaves;
        leaf_idx * MIN_BLOCK
    }
    #[inline]
    fn offset_to_index(off: usize, leaves: usize) -> usize {
        leaves - 1 + off / MIN_BLOCK
    }

    /* ---------- tiny query helpers (unchanged APIs) -------------------- */

    pub fn offset(&self, key: K) -> Option<usize> {
        self.slot_indices.get(key).map(|&(off, _)| off)
    }
    pub fn size(&self, key: K) -> Option<usize> {
        self.slot_indices.get(key).map(|&(_, sz)| sz)
    }
}

/// Recursively initialise the subtree rooted at `node` so that every
/// entry stores the size of the largest free block in that subtree.
/// `size` is the total byte size represented by that node.
fn init_full(tree: &mut [usize], node: usize, size: usize) {
    tree[node] = size;
    if size > MIN_BLOCK {
        let half = size / 2;
        let left = node * 2 + 1;
        let right = left + 1;
        init_full(tree, left, half);
        init_full(tree, right, half);
    }
}
