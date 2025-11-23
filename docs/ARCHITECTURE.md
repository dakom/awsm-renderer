# High-Level Architecture

## Overview

**awsm-renderer** is a WebGPU-based renderer that separates geometry processing from material evaluation. Unlike traditional deferred rendering, the G-Buffer stores **zero material properties** - only geometry IDs and surface data. This decoupling means you can run multiple material passes (PBR, toon, unlit, custom BRDFs) over the same G-buffer without re-rendering geometry, freely mixing material types and iterating on shading without expensive re-rasterization.

### Core Philosophy

- **Pay up-front, reap rewards later**: Expensive operations happen during initialization, not per-frame
- **Unified buffers**: Allocation primitives replace thousands of individual GPU buffers with a handful of shared ones
- **Texture pooling**: Batch uploads and organize textures by size/format for efficient GPU access
- **Screen-space material evaluation (opaque)**: Compute passes process each pixel exactly once, bounded by screen resolution not scene complexity
- **Geometry/material decoupling (opaque)**: G-buffer stores IDs, not material properties. Run multiple material passes over the same geometry without re-rendering.

---

## Table of Contents

- [Dynamic Buffer Architecture](#dynamic-buffer-architecture)
  - [DynamicUniformBuffer - For Fixed-Size Data](#dynamicuniformbuffer---for-fixed-size-data)
  - [DynamicStorageBuffer - For Variable-Size Data](#dynamicstoragebuffer---for-variable-size-data)
  - [The Payoff](#the-payoff)
  - [Upload Strategy](#upload-strategy)
- [Render Pipeline](#render-pipeline)
  - [1. Visibility Pass (Opaque Geometry)](#1-visibility-pass-opaque-geometry)
  - [2. Material Pass (Opaque Materials - Compute Shader)](#2-material-pass-opaque-materials---compute-shader)
- [Resource Management](#resource-management)
  - [Bind Groups](#bind-groups)
  - [Texture Pool](#texture-pool)
- [Performance Analysis: MSAA Memory Cost](#performance-analysis-msaa-memory-cost)
  - [Bandwidth vs. Capacity](#bandwidth-vs-capacity)
  - [Our MSAA Cost (4x, 1080p)](#our-msaa-cost-4x-1080p)
  - [Real-World Impact](#real-world-impact)
  - [The 480 MB "Savings" in Context](#the-480-mb-savings-in-context)

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

### Upload Strategy

Both buffer types upload the **entire buffer** each frame, even if only portions changed. This might seem wasteful, but it's actually optimal:

- **CPU→GPU transfers are fast for contiguous data** - essentially a single optimized memcpy operation
- **One large coherent write** beats many small scattered writes (driver overhead dominates for small transfers)
- **Simpler implementation** - no need to track dirty regions or manage partial updates
- **Better driver optimization** - predictable, uniform upload pattern every frame

**Cost is negligible:** Modern PCIe bandwidth can transfer several GB/s. Even with 10MB of buffer data per frame at 60 FPS (600 MB/s), you're using a tiny fraction of available bandwidth.

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

**MSAA Architecture:**
- All G-buffer textures (including visibility data) are **4x multisampled** and fully stored to VRAM
- **Why not use resolve targets?** The visibility data texture contains **discrete integer IDs** (triangle indices, buffer offsets). Hardware MSAA resolve averages samples, which would corrupt these IDs into invalid values
- **The fundamental constraint:** You cannot have all three simultaneously:
  1. ✅ Keep depth 4x MSAA (for quality)
  2. ✅ Preserve discrete IDs in visibility_data (for material lookups)
  3. ✅ Save bandwidth/storage (via resolve targets)

  **Pick two:**
  - **(1+2)**: Current approach - everything 4x MSAA, fully stored (~320MB at 1080p)
  - **(1+3)**: Resolve visibility_data - IDs corrupted by averaging, material system breaks
  - **(2+3)**: Single-sample rendering - no MSAA, use TAA or other techniques instead
- **Our choice:** (1+2) - Prioritize quality and architectural flexibility at the cost of memory/bandwidth

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
- **Mipmapping via geometry derivatives** - barycentric derivatives from geometry pass drive texture LOD selection, exactly as hardware does in fragment shaders (but with full control)
- Evaluates materials and lighting based on material type

**MSAA Handling:**
- All G-buffer textures (visibility, barycentric, normals, derivatives, depth) remain multisampled
- **Intelligent edge detection** using depth variation and normal discontinuity:
  - **Interior pixels:** Read sample 0 only → single material evaluation (1× cost)
  - **Edge pixels:** Read all 4 samples → per-sample material evaluation with full shading (4× cost)
- **Per-sample shading on edges:** Each MSAA sample can hit a different triangle/material, requiring separate:
  - Triangle ID lookup and material parameters
  - Barycentric interpolation for UVs
  - Texture sampling with proper mipmapping gradients
  - Full lighting evaluation
- **Quality benefit:** Accurate antialiasing at material boundaries where different surfaces meet
- **Performance benefit:** Most pixels are non-edges, so majority of screen processes at 1× cost despite 4× MSAA

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

---

## Performance Analysis: MSAA Memory Cost

### Bandwidth vs. Capacity

Understanding the distinction between these two GPU memory metrics is crucial:

**Bandwidth** = Transfer speed (GB/second)
- How fast data moves between VRAM and GPU cores
- Modern GPUs: 100-700 GB/s depending on tier
- Affects frame time if you're moving too much data per frame

**Capacity** = Storage space (GB total)
- Physical VRAM available for all resources
- Desktop GPUs: 6-24 GB (modern), 1-2 GB (2010-era)
- Web renderers: Constrained by download time, not just VRAM

### Our MSAA Cost (4x, 1080p)

**Per-frame bandwidth:**
- Write (geometry pass): ~320 MB
- Read (material pass): ~100-120 MB (edges only, with intelligent sampling)
- **Total: ~440 MB/frame → 26 GB/s at 60 FPS**

**Permanent capacity:**
- G-buffer storage: **320 MB**

### Real-World Impact

| Platform | Bandwidth Available | Our Usage | Capacity | Our G-Buffer | Verdict |
|----------|---------------------|-----------|----------|--------------|---------|
| **RTX 4060** (2024) | 272 GB/s | 9% | 8 GB | 4% | Negligible ✅ |
| **RTX 4070** (2024) | 504 GB/s | 5% | 12 GB | 2.7% | Negligible ✅ |
| **M1** (2020) | 68 GB/s | 38% | 8 GB shared | 4% | Perfectly fine ✅ |
| **GTX 480** (2010) | 177 GB/s | 15% | 1.5 GB | 21% | Still viable ✅ |

**For web renderers specifically:**
- Typical texture budget: 100-300 MB (not gigabytes)
- Initial page load constraint: Can't download GBs of assets anyway
- Our 320 MB G-buffer is **equivalent to "a few high-res textures" worth** of VRAM
- Trade-off: Material flexibility vs. optimization you'd get from better texture compression or asset reuse

### The 480 MB "Savings" in Context

If we used hardware resolve targets (at the cost of losing discrete IDs):
- **Bandwidth savings**: ~480 MB/frame → ~29 GB/s saved
  - Still only 15-20% of modern GPU bandwidth
  - Bandwidth is plentiful on modern hardware
- **Capacity savings**: ~240 MB
  - Equivalent to 2-3 fewer high-res textures
  - Meaningful but in "asset optimization" territory, not architectural

**Conclusion:** The ID-based G-buffer's memory cost is acceptable for web-scale renderers. Modern GPUs have sufficient bandwidth and VRAM for our approach, while web asset budgets naturally limit total resource usage. The architectural flexibility gained (geometry/material decoupling, multiple material passes without re-rendering geometry, stable G-buffer structure that never needs restructuring) justifies the cost.
