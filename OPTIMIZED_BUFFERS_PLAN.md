# Optimized Buffer Updates: Ergonomics + Performance Assessment

## Context
Current GPU writes use full-buffer uploads whenever a buffer is marked dirty.
Key paths:
- Transforms: `crates/renderer/src/transforms.rs:223`
- Instances: `crates/renderer/src/instances.rs:171`
- Mesh geometry/attributes/meta: `crates/renderer/src/mesh/meshes.rs:622` and `crates/renderer/src/mesh/meta.rs:62`

This is straightforward but potentially wasteful for large scenes where only a small subset changes per frame.

---

## Performance Assessment

### Full-buffer writes (current behavior)
**Pros**
- Simple, predictable performance; few API calls.
- Good when most of the buffer is changing (dense updates).
- Avoids bookkeeping for dirty ranges.

**Cons**
- Wastes bandwidth when updates are sparse.
- Scales poorly with scene size for frequently updated buffers (transforms, instances).
- On discrete GPUs, CPU→GPU transfers can become the bottleneck; on integrated GPUs, memory bandwidth pressure rises.

**Where it’s acceptable:**
- Static buffers (mesh geometry, attribute data, indices) updated rarely.
- Small buffers.

**Where it’s risky:**
- Per-frame transform/instance updates when only a subset is dirty.

### Partial range writes (stream only dirty ranges)
**Pros**
- Reduces bandwidth for sparse updates.
- Scales better with large scenes when only a subset changes.

**Cons**
- Requires tracking dirty ranges per key or per buffer.
- Multiple writes may increase CPU overhead; must coalesce ranges.
- Implementation complexity: need offsets + sizes and per-frame range aggregation.

**Practical middle ground:**
- Track dirty ranges per frame, coalesce to a handful of writes.
- Keep full-buffer writes for large dirty ratios (>50–70%).

---

## Ergonomics Assessment

### Current API
- Simple: mark dirty + call `write_gpu()` once.
- Few knobs to manage.
- Works well for small to medium projects.

### Partial updates ergonomics
- Requires new APIs to mark ranges dirty, probably at the buffer abstraction level.
- Needs a plan for how higher-level systems (Transforms, Instances, MeshMeta) produce offsets.
- More mental overhead: “did I mark the right range?” and “did I coalesce?”

### Recommended ergonomic shape
- Add per-buffer dirty-range tracking inside buffer types.
- Expose **high-level calls**:
  - `Transforms::set_local` and `Instances::transform_update` mark dirty ranges internally.
  - `write_gpu()` decides whether to do full or partial updates.
- Keep API usage unchanged for callers.

---

## Mapped Buffers vs Queue.writeBuffer

### Mapped buffers
**Pros**
- Can be efficient for large streaming writes.
- Useful when you want to write via CPU into a staging buffer and then copy.

**Cons**
- WebGPU mapping can be asynchronous and may stall if not double-buffered.
- Requires explicit synchronization and staging buffers (more complexity).
- Not necessarily faster than `queue.writeBuffer` for moderate update sizes.

### Queue.writeBuffer (current)
**Pros**
- Simple to use; low cognitive overhead.
- Supports partial updates directly with offsets.
- Often sufficient for small/medium update sizes.

**Cons**
- Full-buffer writes are wasteful when updates are sparse.

### Recommendation
- Prefer `queue.writeBuffer` with dirty-range tracking before adding mapped buffers.
- Consider mapped buffers only if you need:
  - Very large per-frame streaming data (e.g., particles, skinned meshes with huge counts)
  - Tight control over staging and synchronization

---

## Proposed Optimization Plan (No Code Yet)

### Phase 1: Dirty Range Tracking for Transforms + Instances
- Track dirty ranges in `DynamicUniformBuffer` and `DynamicStorageBuffer`:
  - When `update_with_unchecked` or `update`, mark (offset, size).
  - Coalesce ranges per frame into a small set.
- In `Transforms::write_gpu` and `Instances::write_gpu`, choose:
  - If dirty_ratio > threshold, upload entire buffer
  - Else upload coalesced ranges

### Phase 2: Mesh Meta Partial Updates
- MeshMeta updates are per mesh; offsets already known.
- Track dirty ranges per meta buffer and write only changed regions.

### Phase 3: Mesh Geometry/Attribute Buffers
- Keep full-buffer writes for geometry/attribute data unless profiling shows bottlenecks.
- These are typically static; the complexity is not worth it unless dynamic meshes are common.

---

## Risk + Complexity Notes
- Dirty-range tracking adds bookkeeping and must stay correct when buffers resize.
- Coalescing algorithm must handle overlapping and adjacent ranges.
- Need to reset dirty ranges after successful write.

---

## Summary
- For typical games, the current full-buffer writes are OK **until** large scenes with sparse updates.
- The most impactful optimization is **partial writes for transforms and instances**.
- Mapped buffers are a later optimization; they’re more complex and not required for most workloads.
- An ergonomic design should keep caller APIs unchanged and handle dirty tracking internally.
