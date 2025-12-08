# Indexed Data Optimization Plan

## Phase 0 Status: ✅ COMPLETE

**Joints/Weights have been removed from the custom attributes system.**
- No longer stored as `MeshBufferCustomVertexAttributeInfo` variants
- Filtered out during glTF loading (mesh.rs:70-77)
- Only stored in skin storage buffers
- **Memory saved: ~320 KB per 10K vertex skinned mesh**

## Current State Analysis

### What's Exploded (Duplicated per Triangle Corner)

1. **Visibility Geometry Vertices** (`visibility.rs`)
   - **STATUS: NECESSARILY EXPLODED** ✓
   - Contains: Position, Normal, Tangent, **Triangle Index**, **Barycentric Coords**
   - **WHY NECESSARY**: Triangle index and barycentric coordinates are unique per corner and cannot be shared
   - Each vertex: 52 bytes (pos:12 + tri_idx:4 + bary:8 + norm:12 + tang:16)
   - Used by: Geometry pass → visibility buffer → material compute pass

2. **Morphs** (`morph.rs`) - **TARGET FOR PHASE 2**
   - **STATUS: UNNECESSARILY EXPLODED** ❌
   - Contains: Position deltas, Normal deltas, Tangent deltas (per morph target)
   - **WHY UNNECESSARY**: These are just offsets applied to base geometry - can be indexed
   - Each corner per target: 40 bytes (pos_delta:12 + norm_delta:12 + tang_delta:16)
   - With 4 morph targets: 160 bytes per corner → **480 bytes for a shared vertex**
   - Used by: Geometry pass compute shader

3. **Skins** (`skin.rs`) - **TARGET FOR PHASE 1**
   - **STATUS: UNNECESSARILY EXPLODED** ❌
   - Contains: Joint indices (4xu32) + Joint weights (4xf32)
   - **WHY UNNECESSARY**: These define how a vertex is influenced by bones - can be indexed
   - Each corner per set: 32 bytes (indices:16 + weights:16)
   - With 2 skin sets: 64 bytes per corner → **192 bytes for a shared vertex**
   - Used by: Geometry pass compute shader

### What's Already Indexed (Kept Compact)

1. **Transparency Geometry Vertices** (`transparency.rs`)
   - **STATUS: CORRECTLY INDEXED** ✓
   - Contains: Position, Normal, Tangent (NOT exploded)
   - Uses original index buffer for vertex sharing
   - Used by: Transparent fragment shader

2. **Custom Attributes** (`attributes.rs`, `mesh.rs`)
   - **STATUS: CORRECTLY INDEXED** ✓
   - Contains: UVs, Colors
   - Stored per original vertex, looked up via indices in shaders
   - Used by: Material compute pass (opaque) and material fragment shader (transparent)

## Memory Savings Calculation

### Example Mesh (Typical Character Model)
- 10,000 original vertices
- 18,000 triangles (54,000 corners after explosion)
- Average vertex valence: 6 triangles per vertex
- 4 morph targets (facial animation)
- 1 skin set (body skinning)

**Current State (Phase 0 Complete):**
- Morphs (storage): 54,000 corners × 4 targets × 40 bytes = **8,640,000 bytes (8.6 MB)**
- Skins (storage): 54,000 corners × 1 set × 32 bytes = **1,728,000 bytes (1.7 MB)**
- **Total: 10.3 MB** (was 10.64 MB before Phase 0)

**After Phase 1 (Index Skins):**
- Morphs (storage): 8.6 MB (unchanged)
- Skins (storage): 10,000 vertices × 1 set × 32 bytes = **320,000 bytes (0.3 MB)** ✓
- **Total: 8.9 MB**
- **Savings: 1.4 MB from skin indexing**

**After Phase 2 (Index Morphs):**
- Morphs (storage): 10,000 vertices × 4 targets × 40 bytes = **1,600,000 bytes (1.6 MB)** ✓
- Skins (storage): 0.3 MB (already indexed)
- **Total: 1.9 MB**
- **Additional savings: 7.0 MB from morph indexing**

**Total Savings from All Phases:**
- Phase 0 (complete): 0.34 MB saved ✅
- Phase 1 (pending): 1.4 MB additional
- Phase 2 (pending): 7.0 MB additional
- **Total: 8.74 MB (82.1% reduction from original 10.64 MB)**

**Bandwidth impact after all phases:**
- Original: 10.64 MB per frame for animated character
- Current (Phase 0 done): 10.3 MB per frame
- After Phase 1: 8.9 MB per frame
- After Phase 2: 1.9 MB per frame
- **Final: ~5.6x less VRAM bandwidth consumed**

## Implementation Plan

### Phase 1: Skin Data Indexing (NEXT PRIORITY)

**Priority: HIGH** - Affects all skinned meshes, simpler to implement than morphs

#### 1.1 Update Skin Storage Format (`gltf/buffers/skin.rs`)

**Current (Exploded):**
```rust
// For each triangle corner: [indices:16, weights:16] × set_count
// 54,000 corners × 32 bytes = 1.7 MB
```

**Proposed (Indexed):**
```rust
// For each original vertex: [indices:16, weights:16] × set_count
// 10,000 vertices × 32 bytes = 320 KB
```

**Changes:**
- Remove explosion loop (lines 83-107)
- Store one entry per original vertex instead of per corner
- Keep same data format: standardized u32 indices + f32 weights

**File: `crates/renderer/src/gltf/buffers/skin.rs`**

```rust
// Remove:
// - Triangle extraction (line 78)
// - Triangle loop (lines 83-107)

// Replace with:
for vertex_index in 0..original_vertex_count {
    for skin_set_data in &skin_sets_data {
        let indices_u32 = convert_indices_to_u32(..., vertex_index)?;
        let weights_f32 = convert_weights_to_f32(..., vertex_index)?;

        for i in 0..4 {
            skin_joint_index_weight_bytes.extend_from_slice(&indices_u32[i].to_le_bytes());
            skin_joint_index_weight_bytes.extend_from_slice(&weights_f32[i].to_le_bytes());
        }
    }
}
```

**Struct changes:**
```rust
// In MeshBufferSkinInfoWithOffset
pub struct MeshBufferSkinInfoWithOffset {
    pub set_count: usize,
    pub index_weights_offset: usize,
    pub index_weights_size: usize,
    // Add: (or infer from other mesh data)
    pub original_vertex_count: usize,  // NEW: for shader to validate bounds
}
```

#### 1.2 Update Geometry Pass Shader to Use Indexed Lookup

**File: `crates/renderer/src/render_passes/geometry/shader/geometry_wgsl/skin.wgsl`**

**Current (Exploded - Direct Access):**
```wgsl
// Skin data is already exploded to match triangle corner index
let skin_offset = meta.skin_offset + (corner_index * SKIN_STRIDE_PER_CORNER);
let joint_indices = read_joint_indices(skin_offset);
let joint_weights = read_joint_weights(skin_offset);
```

**Proposed (Indexed - Indirect Access):**
```wgsl
// Read vertex index from triangle data, then lookup skin data
let vertex_index = triangle_data[corner_index];  // or however you get original vertex index
let skin_offset = meta.skin_offset + (vertex_index * SKIN_STRIDE_PER_VERTEX);
let joint_indices = read_joint_indices(skin_offset);
let joint_weights = read_joint_weights(skin_offset);
```

**Key consideration:** You already have access to the original vertex indices through the triangle index buffer or attribute indices. Use those to look up skin data.

#### 1.3 Update Metadata and Offsets

**Files to update:**
- `crates/renderer/src/mesh/skins.rs` - Update `SkinGpuInfo` if needed
- `crates/renderer/src/mesh/meta/*.rs` - Update meta structs if they store skin strides

**Changes:**
- Update `SKIN_STRIDE` constants from per-corner to per-vertex
- Update any offset calculations that assumed exploded format

### Phase 2: Morph Data Indexing

**Priority: MEDIUM** - Affects meshes with morph targets, more complex than skins due to multiple attributes

#### 2.1 Update Morph Storage Format (`gltf/buffers/morph.rs`)

**Current (Exploded):**
```rust
// For each triangle corner, all targets: [T0_pos:12, T0_norm:12, T0_tang:16, T1_pos:12, ...]
// 54,000 corners × 4 targets × 40 bytes = 8.6 MB
```

**Proposed (Indexed):**
```rust
// For each original vertex, all targets: [T0_pos:12, T0_norm:12, T0_tang:16, T1_pos:12, ...]
// 10,000 vertices × 4 targets × 40 bytes = 1.6 MB
```

**File: `crates/renderer/src/gltf/buffers/morph.rs`**

**Changes:**
- Remove triangle explosion loop (lines 122-187)
- Store one entry per original vertex
- Maintain same interleaving pattern (all targets per vertex)

```rust
// Remove:
// - Triangle extraction (line 99)
// - Triangle loops (lines 122-187)

// Replace with:
for vertex_index in 0..original_vertex_count {
    for morph_target_buffer_data in &morph_targets_buffer_data {
        // Position delta (12 bytes)
        match &morph_target_buffer_data.positions {
            Some(position_data) => {
                let offset = vertex_index * 12;
                geometry_morph_bytes.extend_from_slice(&position_data[offset..offset + 12]);
            }
            None => geometry_morph_bytes.extend_from_slice(slice_zeroes(12)),
        }

        // Normal delta (12 bytes)
        // ... same pattern

        // Tangent delta (12 bytes from GLTF, pad to 16)
        // ... same pattern
    }
}
```

**Struct changes:**
```rust
// In MeshBufferGeometryMorphInfoWithOffset
pub struct MeshBufferGeometryMorphInfoWithOffset {
    pub targets_len: usize,
    pub vertex_stride_size: usize,  // CHANGED: was triangle_stride_size
    pub values_size: usize,
    pub values_offset: usize,
    // Add: (or infer from other mesh data)
    pub original_vertex_count: usize,  // NEW: for validation
}
```

#### 2.2 Update Geometry Pass Shader to Use Indexed Lookup

**File: `crates/renderer/src/render_passes/geometry/shader/geometry_wgsl/morph.wgsl`**

**Current (Exploded - Direct Access):**
```wgsl
// Morph data is already exploded to match triangle corner index
let morph_offset = meta.morph_offset + (corner_index * MORPH_STRIDE_PER_CORNER);
for (var target_idx = 0u; target_idx < target_count; target_idx++) {
    let target_offset = morph_offset + (target_idx * MORPH_TARGET_SIZE);
    pos_delta += read_morph_position(target_offset) * weights[target_idx];
    // ... same for normal and tangent
}
```

**Proposed (Indexed - Indirect Access):**
```wgsl
// Read vertex index, then lookup morph data
let vertex_index = triangle_data[corner_index];  // Get original vertex index
let morph_offset = meta.morph_offset + (vertex_index * MORPH_STRIDE_PER_VERTEX);
for (var target_idx = 0u; target_idx < target_count; target_idx++) {
    let target_offset = morph_offset + (target_idx * MORPH_TARGET_SIZE);
    pos_delta += read_morph_position(target_offset) * weights[target_idx];
    // ... same for normal and tangent
}
```

#### 2.3 Update Metadata and Offsets

**Files to update:**
- `crates/renderer/src/mesh/morphs.rs` - Update morph metadata structs
- `crates/renderer/src/mesh/meta/*.rs` - Update stride calculations

**Changes:**
- Update stride constants from per-corner to per-vertex
- Update any offset calculations that assumed exploded format

### Phase 3: Shader Integration

#### 3.1 Add Vertex Index Lookup Helper

**File: `crates/renderer/src/render_passes/geometry/shader/geometry_wgsl/helpers/vertex_lookup.wgsl` (NEW)**

```wgsl
// Get the original vertex index for a given triangle corner
// This is needed to look up indexed data (morphs, skins) from exploded visibility buffer
fn get_original_vertex_index(
    triangle_index: u32,
    corner_index_in_triangle: u32,  // 0, 1, or 2
    triangle_data: /* appropriate type */
) -> u32 {
    // triangle_data contains the original vertex indices for each triangle
    // Format depends on how triangle indices are stored
    // Could be: triangle_data[triangle_index * 3 + corner_index_in_triangle]
    // Or might need to read from triangle attribute indices buffer

    // This implementation depends on your current triangle data structure
}
```

**Implementation note:** You'll need to determine where the original vertex indices are available in the geometry pass. They might be:
- In the triangle data buffer (if you're storing them)
- In the attribute indices buffer (if that's per-vertex)
- Reconstructable from the visibility buffer metadata

#### 3.2 Update Geometry Pass Main Shader

**File: `crates/renderer/src/render_passes/geometry/shader/geometry_wgsl/vertex.wgsl`**

```wgsl
// Before: Direct access using exploded corner index
let corner_index = @builtin(vertex_index);  // or similar

// After: Get original vertex index for indexed lookups
let corner_index = @builtin(vertex_index);
let vertex_index = get_original_vertex_index(triangle_index, corner_in_tri, ...);

// Use corner_index for exploded data (position, normal, tangent from visibility buffer)
let position = visibility_buffer[corner_index].position;

// Use vertex_index for indexed data (morphs, skins)
if (has_morphs) {
    let morph_data = read_morph(vertex_index, ...);
    position += apply_morphs(morph_data, ...);
}
if (has_skin) {
    let skin_data = read_skin(vertex_index, ...);
    position = apply_skinning(position, skin_data, ...);
}
```

### Phase 4: Testing and Validation

#### 4.1 Unit Tests

Create tests to verify:
- Indexed skin data produces same results as exploded
- Indexed morph data produces same results as exploded
- Vertex index lookup is correct for all corners
- Boundary conditions (last vertex, first triangle, etc.)

**File: `crates/renderer/src/gltf/buffers/tests.rs` (NEW or extend existing)**

```rust
#[test]
fn test_skin_indexed_matches_exploded() {
    // Load same mesh with both implementations
    // Verify byte-for-byte equivalence of results
}

#[test]
fn test_morph_indexed_matches_exploded() {
    // Similar test for morphs
}
```

#### 4.2 Integration Tests

Test with actual glTF files:
- RecursiveSkeletons.gltf (skinning)
- AnimatedMorphCube.gltf (morphs, if available)
- Complex character models with both skins and morphs

**Validation:**
- Visual inspection: Does animation look identical?
- Performance metrics: Is it faster?
- Memory usage: Verify expected reduction

#### 4.3 Performance Benchmarking

Measure before/after:
- VRAM usage (should decrease by ~80%)
- Frame time (should improve slightly)
- GPU memory bandwidth (use profiler like RenderDoc/PIX)
- CPU→GPU transfer time (should improve)

### Phase 5: Cleanup and Documentation

#### 5.1 Remove Old Code

Once validated:
- Remove explosion loops from `skin.rs` and `morph.rs`
- Remove any "exploded" comments that are now incorrect
- Update all doc comments to reflect indexed format

#### 5.2 Update Documentation

**Files to update:**
- `docs/ARCHITECTURE.md` - Update geometry pass description
- `docs/MEMORY_LAYOUT.md` - Update buffer layout diagrams (if exists)
- Code comments in all modified files
- This plan document (mark as COMPLETED)

**Key points to document:**
- Why visibility buffer must stay exploded (triangle_index + bary)
- Why morphs/skins can be indexed (they're per-vertex properties)
- How vertex index lookup works in shaders
- Performance gains achieved

## Implementation Order

### ✅ Phase 0: Remove Duplicate Attributes - COMPLETE
- Joints/Weights removed from custom attributes
- 0.34 MB saved per 10K vertex skinned mesh

### 🎯 Next Steps

1. **Phase 1: Index Skins** (RECOMMENDED NEXT - simpler, single data type)
   - Implement indexed storage (1-2 days)
   - Update geometry shader (1 day)
   - Test with RecursiveSkeletons.gltf (1 day)
   - **Expected savings: 1.4 MB per example mesh**

2. **Phase 2: Index Morphs** (more complex, multiple interleaved attributes)
   - Implement indexed storage (2-3 days)
   - Update geometry shader (1-2 days)
   - Test with morph target models (1 day)
   - **Expected savings: 7.0 MB per example mesh**

3. **Phase 3: Optimize and Polish** (1-2 days)
   - Performance benchmarking
   - Memory profiling
   - Documentation updates

**Remaining estimated time: 4 + 5 + 2 = 11 days**

## Risks and Mitigations

### Risk 1: Vertex Index Availability
**Problem:** Original vertex indices might not be readily available in geometry shader.

**Mitigation:**
- Check current triangle data structure
- If needed, store vertex indices alongside triangle data
- Minimal overhead (4 bytes per corner = 12 bytes per triangle)

### Risk 2: Cache Coherency
**Problem:** Indexed access might reduce cache hit rate if vertices accessed randomly.

**Mitigation:**
- Modern GPUs have large caches and prefetchers
- Multiple corners often access nearby vertices (spatial locality)
- The bandwidth savings far outweigh potential cache misses
- Profile to verify (but unlikely to be an issue)

### Risk 3: Shader Complexity
**Problem:** Adding indirection makes shaders slightly more complex.

**Mitigation:**
- Well-documented helper functions
- Clear separation between exploded (visibility) and indexed (morph/skin) data
- Complexity is minimal (one extra index read)

### Risk 4: Breaking Changes
**Problem:** Existing content might break during migration.

**Mitigation:**
- Implement behind feature flag initially
- Extensive testing with existing glTF files
- Keep old code until fully validated
- Document migration path clearly

## Success Metrics

### Phase 0 (Complete) ✅
- [x] Joints/Weights removed from custom attributes
- [x] No visual changes
- [x] Memory reduced by ~320 KB per 10K vertex skinned mesh
- [x] Clean architectural separation maintained

### Phases 1-2 (Remaining)

#### Must Have
- [ ] Visual output identical to current implementation
- [ ] All existing glTF test files render correctly
- [ ] No GPU errors or validation warnings

#### Performance Goals
- [ ] VRAM usage reduced by >70% for skinned/morphed meshes (from original baseline)
- [ ] Frame time improved or equal (should not regress)
- [ ] CPU→GPU transfer time reduced by >70%

#### Code Quality
- [ ] All code properly documented
- [ ] No increase in shader complexity (subjective)
- [ ] Test coverage for new code paths

## Future Optimizations

After indexed data is working:

1. **Compress Morph Targets**
   - Currently storing full f32 deltas
   - Could use f16 or quantized formats
   - Additional 50% savings possible

2. **Optimize Skin Storage**
   - Many vertices use <4 joints
   - Could use variable-length storage
   - Additional 30-40% savings for typical meshes

3. **GPU-Side Compression**
   - Use texture compression for morph targets
   - BC4/BC5 compression for normal/tangent deltas
   - Requires shader decompression but saves bandwidth

4. **Indexed Transparency Geometry**
   - Currently transparency geometry includes position/normal/tangent
   - Could reference visibility geometry data instead
   - Reduces duplication between opaque and transparent passes

## References

- Current implementation: `gltf/buffers/skin.rs`, `gltf/buffers/morph.rs`
- Geometry pass shaders: `render_passes/geometry/shader/geometry_wgsl/`
- Similar indexed approach: `gltf/buffers/attributes.rs` (custom attributes)
