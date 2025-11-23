# High-Level Architecture

## Overview

**awsm-renderer** is a WebGPU-based renderer that separates geometry processing from material evaluation. Unlike traditional deferred rendering, the G-Buffer stores **zero material properties** - only geometry IDs and surface data. This gives you complete freedom to mix any material models (PBR, toon, unlit, custom BRDFs) in the same scene.

### Core Philosophy

- **Pay up-front, reap rewards later**: Expensive operations happen during initialization, not per-frame
- **Unified buffers**: Allocation primitives replace thousands of individual GPU buffers with a handful of shared ones
- **Texture pooling**: Batch uploads and organize textures by size/format for efficient GPU access
- **Bandwidth is precious**: e.g. aggressive MSAA optimization saves ~480MB/frame at 1080p

---

## Dynamic Buffer Architecture

**Philosophy:** One unified buffer beats a thousand small ones.

**The problem:** Traditional approaches create individual GPU buffers for each mesh, material, light, transform, etc. This leads to:
- Thousands of bind group creations and recreations
- Expensive GPU buffer allocation/deallocation per object
- Bind group layout explosion (one per object type)
- Poor memory locality and cache behavior

**Our solution:** Two specialized dynamic buffer types that handle **all** object data in unified buffers:

### DynamicUniformBuffer - For Fixed-Size Data

**Use for:** Transforms, lights, PBR materials - anything where all items are the same size.

**Characteristics:**
- **O(1) insert/update/remove** (amortized) - constant time operations
- **Zero fragmentation** - fixed-size slots with perfect reuse
- **Slot recycling** - removed items' slots immediately available for reuse
- **Minimal overhead** - just slot index tracking, no complex allocation logic

**Performance:**
```
• CPU operations:    O(1) all operations
• Memory efficiency: 100% (no internal fragmentation)
• Slot reuse:        Immediate (via free list)
• GPU uploads:       Full buffer per frame (unified write)
```

**Example:** Managing 10,000 transforms in a single buffer instead of 10,000 individual buffers.

### DynamicStorageBuffer - For Variable-Size Data

**Use for:** Morph weights, skin joints, mesh attributes - data with heterogeneous sizes.

**Characteristics:**
- **O(log N) insert/update/remove** (amortized) - buddy allocation tree traversal
- **Zero external fragmentation** - buddy blocks always coalesce
- **≤50% internal fragmentation** - power-of-two rounding per allocation
- **Automatic growth** - buffer doubles when needed

**Performance:**
```
• CPU operations:    O(log N) with buddy tree
• Memory efficiency: 50-100% per allocation (power-of-two rounding)
• Block coalescing:  Automatic (buddy algorithm)
• GPU uploads:       Full buffer per frame (unified write)
```

**Example:** Storing morph target weights where mesh A needs 8 targets (32 bytes) and mesh B needs 128 targets (512 bytes).

### The Payoff

**Before (per-object buffers):**
- 1,000 meshes = 1,000 GPU buffers + 1,000 bind groups
- Every add/remove triggers GPU buffer create/destroy
- Bind group chaos - constant recreation on updates

**After (unified buffers):**
- 1,000 meshes = 1 GPU buffer + 1 bind group
- Add/remove is pure CPU bookkeeping (O(1) or O(log N))
- Single bind group persists - recreate only on buffer growth (rare)
- All data uploaded together - better locality and batching

**Real-world impact:**
- Bind group count: **1,000x reduction** (one per buffer type, not per object)
- GPU buffer allocations: **1,000x reduction**
- Per-frame overhead: **Pure CPU allocation** (no GPU calls except data upload)
- Memory layout: **Contiguous and cache-friendly**

---

## Render Pipeline

### 1. Visibility Pass (Opaque Geometry)

**Purpose:** Transform and rasterize geometry - **no material evaluation**.

**What it does:**
- Vertex transformations: positions, normals, tangents (including skinning and morph targets)
- Fast rasterization: just vertex processing + triangle rasterization
- Outputs pure geometry data - **what** was hit (triangle/mesh IDs), **where** on the surface (barycentrics), and surface math for shading (normals/tangents/derivatives)

**G-Buffer Outputs (4 color + depth):**

| Texture | Format | Contents |
|---------|--------|----------|
| **Visibility Data** | `RGBA16uint` | Triangle ID (xy) + Mesh Meta Offset (zw) - identifies **what** was hit |
| **Barycentric** | `RG16float` | Barycentric coordinates (x, y) - identifies **where** on triangle |
| **Normal/Tangent** | `RGBA16float` | Octahedral normal (2ch) + tangent angle (1ch) + handedness (1ch) - surface geometry |
| **Bary Derivatives** | `RGBA16float` | ddx barycentric (2ch) + ddy barycentric (2ch) - for texture LOD calculation |
| **Depth** | `Depth24plus` | Standard depth buffer |

**MSAA Optimization:**
- Rasterization to **4x multisampled render targets** (kept in tile memory)
- Hardware **MSAA resolve** to single-sample textures (averaging done by GPU)
- **`StoreOp::Discard`** on color attachments - multisampled intermediates never written to VRAM
- **Bandwidth savings: ~480MB/frame at 1080p**
  - ~240MB write eliminated (discard operation)
  - ~240MB read eliminated (compute reads single-sample)
- Depth remains multisampled for future depth-aware effects

---

### 2. Material Pass (Opaque Materials - Compute Shader)

**Purpose:** Evaluate materials and lighting only for visible pixels.

**Why compute shader?** Because it fundamentally changes the performance equation.

**Traditional fragment shading (per-mesh):**
```
Cost = Σ(fragments per mesh) × material_cost
     = Depends on draw order, overdraw, and per-mesh processing
```

Even with early-Z helping for opaque geometry, you still have:
- **Draw order dependency** - front-to-back sorting helps, but never perfect
- **Per-mesh overhead** - vertex processing, rasterization, and setup for every mesh
- **Material switching** - pipeline/bind group changes between different materials
- **Redundant work** - same pixel evaluated multiple times if draws aren't perfectly sorted

**Our compute approach (per-pixel):**
```
Cost = screen_pixels × active_material_types
     = At most 2M pixels @ 1080p
```

**The massive win:**
- **Upper bound is screen resolution** - 1920×1080 = 2M pixels maximum, regardless of scene complexity
- **Process each pixel exactly once** per material type - no draw order dependency, no redundant evaluations
- **Tile-level early-exit** - entire tiles skip materials they don't contain (zero cost)
- **No per-mesh overhead** - one dispatch per material type, not per mesh
- **Perfect cache coherency** - processing screen-space tiles, not scattered mesh fragments

**Example:** Scene with 1000 meshes using 3 material types:
- Traditional: Vertex + fragment processing for all 1000 meshes, order-dependent overdraw
- Our approach: Visibility pass for all 1000 meshes (cheap, geometry-only), then 3 material dispatches over screen pixels with tile early-exit
- **Win scales with scene complexity** - more meshes = more win, since material cost stays bounded by screen size

**What it does:**
- Reads G-buffer data (IDs, barycentrics, normals, derivatives)
- Uses IDs to look up material parameters from storage buffers
- Manual mipmapping using barycentric derivatives
- Evaluates materials and lighting based on material type

**Output:**
- Single `RGBA16float` texture: final color (lit or unlit, depending on material)

---

## Resource Management

### Bind Groups

**Philosophy:** Setup is expensive, data is cheap.

**Key insights:**
- Pipeline and bind group creation costs dominate
- Multiple data sources can affect multiple bind groups
- Buffer resizes are the primary cause of bind group invalidation

**Strategy:**
- Set up bind groups **up-front** with maximum anticipated data
- Signal bind group updates **lazily** - rebuild later, not immediately
- Load data up-front to minimize bind group recreation

---

### Texture Pool

**Architecture:** Textures organized by size and format into texture arrays.

**Why pooling?**
- GPU texture upload is expensive
- Affects pipelines and shader bindings
- Batch operations minimize pipeline churn

**Workflow:**
1. **Add textures** to pool (cheap operation)
2. **Call `finalize_gpu_textures()`** after all additions:
   - Uploads new texture arrays to GPU
   - Rebuilds affected pipelines/shaders
   - Signals bind group recreation

This batching approach dramatically reduces the cost of texture management.
