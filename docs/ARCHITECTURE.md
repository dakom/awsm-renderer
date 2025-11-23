# High-Level Architecture

## Overview

**awsm-renderer** is a WebGPU-based visibility buffer renderer that separates geometry processing from material evaluation. Unlike traditional deferred rendering, it makes **zero assumptions about material models** - giving you complete freedom to use PBR, toon shading, unlit materials, or any custom material in the same scene.

Optimized for performance through smart resource management and aggressive bandwidth optimization.

### Core Philosophy

- **Visibility buffer approach**: G-Buffer stores **zero material properties** - only geometry IDs and surface data
  - Complete material flexibility: PBR, toon/cel-shading, unlit, or any exotic material model
  - Material evaluation happens in compute shader using geometry IDs to look up parameters
- **Pay up-front, reap rewards later**: Expensive operations (texture uploads, pipeline/bind group creation) happen during initialization
- **Bandwidth is precious**: Aggressive optimization to minimize memory traffic (~480MB/frame savings at 1080p)
- **Texture pooling**: Organize textures into arrays by size/format for efficient GPU access

---

## Render Pipeline

### 1. Visibility Pass (Opaque Geometry)

**Purpose:** Transform and rasterize geometry - **no material evaluation**.

**What it does:**
- Vertex transformations: positions, normals, tangents (including skinning and morph targets)
- Fast rasterization: just vertex processing + triangle rasterization
- Outputs **pure geometry/visibility data** - zero material properties

**Why this matters:**
Unlike traditional deferred rendering, the G-Buffer encodes **no assumptions about material models**. It only stores:
- **What was hit** (triangle/mesh IDs)
- **Where on the surface** (barycentric coordinates)
- **Geometric orientation** (normals/tangents for the surface, not material-specific)
- **Texture sampling math** (derivatives for mipmapping)

This gives you **complete material flexibility** - PBR, toon shading, unlit, or any exotic material can be evaluated in the compute pass.

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

**Why compute shader?**
- Operates in screen space over resolved G-buffer textures
- **Material evaluation only happens where geometry was rasterized** - massive performance win
- Early-exit per pixel: "this pixel isn't mine" → zero cost

**What it does:**
- Reads G-buffer data (triangle/mesh IDs, barycentric coords, normals, derivatives)
- **Uses IDs to look up which material to evaluate** - no material assumptions in G-Buffer!
- Loads material parameters from storage buffers (UVs, colors, material-specific settings)
- **Manual mipmapping** using barycentric derivatives from geometry pass
- **Evaluates any material model**: PBR, toon/cel-shading, unlit, custom BRDFs, etc.
- Supports multiple material types via branching or separate pipelines

**Material flexibility:**
Because the G-Buffer stores no material properties, you can:
- Mix PBR and non-photorealistic materials in the same scene
- Add exotic materials without changing the visibility pass
- Switch material models per-mesh based on metadata

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
