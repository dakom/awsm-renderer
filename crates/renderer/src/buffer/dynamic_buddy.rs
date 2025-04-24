use slotmap::{Key, SecondaryMap};

//-------------------------------- PERFORMANCE SUMMARY ------------------------//
//
// • insert/update/remove:   O(log N) (amortized, ignoring rare growth)
// • GPU write (per frame):  uploads entire buffer each time
// • Resize strategy:        doubles capacity when needed; rebuilds tree
//                           (infrequent pauses)
// • External fragmentation: none (buddy blocks always coalesce)
// • Internal fragmentation: ≤ 50% per allocation (due to power-of-two rounding)
// • Memory overhead:        raw_data.len() rounded up + buddy tree (~2× leaves)
//
// • Ideal usage:
//    Mixed-size uniform/storage buffer items where predictable performance
//    matters more than perfect memory efficiency, like:
//      - Heterogeneous UBO/SBO payloads
//      - Variable-sized dynamic allocations
//
//----------------------------------------------------------------------------//

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
    gpu_buffer_needs_resize: bool,
    #[allow(dead_code)]
    label: Option<String>,
}

impl<K: Key, const ZERO: u8> DynamicBuddyBuffer<K, ZERO> {
    pub fn new(mut initial_bytes: usize, label: Option<String>) -> Self {
        // round up to next power‑of‑two multiple of MIN_BLOCK
        initial_bytes = round_pow2(initial_bytes.max(MIN_BLOCK));

        // buddy tree size: 2 * cap / MIN_BLOCK  – 1  (perfect binary tree)
        let leaves = initial_bytes / MIN_BLOCK;
        let mut buddy_tree = vec![0; 2 * leaves - 1];

        init_full(&mut buddy_tree, 0, initial_bytes);

        // CPU
        let raw_data = vec![ZERO; initial_bytes];

        Self {
            raw_data,
            buddy_tree,
            slot_indices: SecondaryMap::new(),
            gpu_buffer_needs_resize: false,
            label,
        }
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

    // careful, just to update existing data that definitely will not grow (or insert)
    pub fn update_with_unchecked(&mut self, key: K, f: impl FnOnce(&mut [u8])) {
        match self.slot_indices.get(key) {
            Some((off, size)) => {
                f(&mut self.raw_data[*off..*off + *size]);
            }
            None => {
                panic!("Key {key:?} not found in DynamicBuddyBuffer");
            }
        }
    }

    fn insert(&mut self, key: K, bytes: &[u8]) {
        let req = round_pow2(bytes.len().max(MIN_BLOCK));
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

    pub fn raw_slice(&self) -> &[u8] {
        &self.raw_data
    }

    pub fn take_gpu_needs_resize(&mut self) -> Option<usize> {
        let size = match self.gpu_buffer_needs_resize {
            true => Some(self.raw_data.len()),
            false => None,
        };

        self.gpu_buffer_needs_resize = false;

        size
    }

    /* ------------------------------------------------------------------ */
    /*                  Buddy tree helpers                                */
    /* ------------------------------------------------------------------ */

    /// Allocate a block of exactly `req` bytes (power‑of‑two, ≥ MIN_BLOCK).
    fn alloc(&mut self, req: usize) -> Option<usize> {
        if req > self.buddy_tree[0] {
            return None;
        }

        let mut idx = 0usize; // start at root
        let mut size = self.raw_data.len(); // current block size

        while size > req {
            let half = size / 2;
            let left = idx * 2 + 1;
            // choose the child that can still satisfy the request
            idx = if self.buddy_tree[left] >= req {
                left
            } else {
                left + 1
            };
            size = half;
        }

        // `idx` now points to the block we want
        self.buddy_tree[idx] = 0;
        fix_parents(&mut self.buddy_tree, idx);

        Some(index_to_offset(idx, self.raw_data.len() / MIN_BLOCK))
    }

    /// Marks a previously‑allocated block `[offset , offset+size)` as free.
    ///
    /// `offset` **must** be the same value returned by `alloc`, and
    /// `size` **must** be a power‑of‑two ≥ MIN_BLOCK (already true because it
    /// comes from `self.slot_indices`).
    fn free(&mut self, offset: usize, size: usize) {
        let leaves = self.raw_data.len() / MIN_BLOCK;

        // ── 1. find the tree‑node that owns this exact block ────────────────
        let mut idx = offset_to_index(offset, leaves); // start at leaf
        let mut blk = MIN_BLOCK; // leaf block size

        while blk < size {
            // climb until we reach the right level
            idx = (idx - 1) >> 1;
            blk <<= 1;
        }
        self.buddy_tree[idx] = blk; // mark that node free

        // ── 2. bubble upward, merging buddies when BOTH are equally free ────
        while idx != 0 {
            let parent = (idx - 1) >> 1;
            let left = parent * 2 + 1;
            let right = left + 1;

            let merged = self.buddy_tree[left] == blk && self.buddy_tree[right] == blk;

            let new_val = if merged {
                blk << 1 // buddies coalesce → parent block size
            } else {
                self.buddy_tree[left].max(self.buddy_tree[right])
            };

            if self.buddy_tree[parent] == new_val {
                break; // nothing changed ⇒ done
            }
            self.buddy_tree[parent] = new_val;

            if merged {
                idx = parent; // continue trying to merge upward
                blk <<= 1;
            } else {
                break; // no merge ⇒ parents are already correct
            }
        }
    }

    fn grow(&mut self, min_extra: usize) {
        let old_cap = self.raw_data.len();
        let mut new_cap = old_cap * 2;
        while new_cap - old_cap < min_extra {
            new_cap *= 2;
        }
        self.raw_data.resize(new_cap, ZERO);
        self.gpu_buffer_needs_resize = true;

        // rebuild a new perfect tree
        let leaves = new_cap / MIN_BLOCK;
        self.buddy_tree.clear();
        self.buddy_tree.resize(2 * leaves - 1, 0);
        init_full(&mut self.buddy_tree, 0, new_cap);

        // re‑insert existing allocations so they become *used* leaves
        for (offset, size) in self.slot_indices.values().cloned() {
            mark_used(&mut self.buddy_tree, self.raw_data.len(), offset, size);
        }
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

#[inline]
fn fix_parents(buddy_tree: &mut [usize], mut idx: usize) {
    while idx != 0 {
        let parent = (idx - 1) >> 1;
        let left = parent * 2 + 1;
        let right = left + 1;
        let new_val = buddy_tree[left].max(buddy_tree[right]);
        if buddy_tree[parent] == new_val {
            break;
        }
        buddy_tree[parent] = new_val;
        idx = parent;
    }
}

fn mark_used(buddy_tree: &mut [usize], raw_data_len: usize, offset: usize, size: usize) {
    let leaves = raw_data_len / MIN_BLOCK;
    let mut idx = offset_to_index(offset, leaves);
    let mut sz = MIN_BLOCK;
    while sz < size {
        idx = (idx - 1) >> 1;
        sz <<= 1;
    }
    buddy_tree[idx] = 0;
    fix_parents(buddy_tree, idx);
}

/* ---- index/offset helpers & math utils ---------------------------- */

#[inline]
fn round_pow2(n: usize) -> usize {
    n.next_power_of_two().max(MIN_BLOCK)
}
#[inline]
fn index_to_offset(mut idx: usize, leaves: usize) -> usize {
    // walk to the left‑most leaf of this subtree
    while idx < leaves - 1 {
        idx = idx * 2 + 1;
    }
    let leaf_idx = idx + 1 - leaves;
    leaf_idx * MIN_BLOCK
}
#[inline]
fn offset_to_index(off: usize, leaves: usize) -> usize {
    leaves - 1 + off / MIN_BLOCK
}
