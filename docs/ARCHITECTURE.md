# High-Level Architecture

## Overview

**awsm-renderer** is a WebGPU-based visibility buffer renderer that separates geometry processing from material evaluation. Unlike traditional deferred rendering, the G-Buffer stores **zero material properties** - only geometry IDs and surface data. This gives you complete freedom to mix any material models (PBR, toon, unlit, custom BRDFs) in the same scene.

### Core Philosophy

- **Pay up-front, reap rewards later**: Expensive operations happen during initialization, not per-frame
- **Bandwidth is precious**: Aggressive MSAA optimization saves ~480MB/frame at 1080p
- **Texture pooling**: Batch uploads and organize textures by size/format for efficient GPU access

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

**Why compute shader?**
- Material evaluation only happens for visible pixels - massive performance win
- Early-exit per pixel for unaffected materials - zero cost

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
