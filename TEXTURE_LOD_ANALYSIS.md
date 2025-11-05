# Complete Texture Atlas and LOD Calculation Pipeline Analysis

## Executive Summary

The system has **critical inconsistencies** in the LOD calculation pipeline that stem from a fundamental mismatch between how derivatives are computed and how they're used. The 0.35 correction factor is a band-aid that masks the real issue: the LOD formula doesn't properly account for atlas transformations, and derivatives are being computed in inconsistent spaces.

---

## 1. Mipmap Generation Pipeline

### Location
`crates/renderer-core/src/texture/mipmap.rs` (lines 50-159)

### Key Implementation Details

**Filter Strategy: Smart Box Filter with Contrast Preservation**

```rust
// Mipmap generation uses a contrast-aware blend:
// - For high-contrast areas (thin lines, grids): preserve brightest sample
// - For low-contrast areas (smooth gradients): use box filter (average 4 samples)

let box_filter = (sample_00 + sample_01 + sample_10 + sample_11) * 0.25;
let max_preserve = max(max(sample_00, sample_01), max(sample_10, sample_11));

// Blend based on luminance contrast
let contrast = max_lum - min_lum;
let preserve_amount = saturate((contrast - contrast_threshold) / contrast_threshold);
let result = mix(box_filter, max_preserve, preserve_amount);
```

**Sampling Configuration (lines 175-180)**
- Uses **linear filtering** (FilterMode::Linear) for smooth mipmap generation
- Bilinear interpolation when downsampling

**Process**
1. For each mip level from 1 to N:
   - Input size: texel_center at `vec2<f32>(global_id.xy) * 2.0 + 0.5`
   - Output size: half of input
   - Sample 4 points from input mip level
   - Apply contrast-preserving blend
   - Store result in output mip level

**Key Metric: `calculate_mipmap_levels()`**
```rust
fn calculate_mipmap_levels(width: u32, height: u32) -> u32 {
    ((width.max(height) as f32).log2().floor() as u32) + 1
}
```
This is correct: for a 256×256 texture, generates 9 mip levels (256, 128, 64, 32, 16, 8, 4, 2, 1).

---

## 2. Texture Atlas Structure

### TextureInfo Structure
Location: `crates/renderer/src/render_passes/material/shared/shader/all_material_shared_wgsl/textures.wgsl` (lines 14-24)

```wgsl
struct TextureInfo {
    pixel_offset: vec2<u32>,           // Top-left corner in atlas (texels)
    size: vec2<u32>,                   // Width/height of texture (texels)
    atlas_index: u32,                  // Which atlas texture this belongs to
    layer_index: u32,                  // Layer in the 2D array
    entry_index: u32,                  // Entry metadata
    attribute_uv_set_index: u32,       // Which UV set to use
    sampler_index: u32,                // Which sampler for this texture
    address_mode_u: u32,               // Clamp/Repeat/Mirror
    address_mode_v: u32,               // Clamp/Repeat/Mirror
}
```

### Local UV → Atlas UV Transformation

**Function: `_texture_sample_atlas()` (lines 95-134)**

```wgsl
// Input: attribute_uv in [0, 1] (local texture coordinates)
// Output: uv in [0, 1] (atlas coordinates)

let wrapped_uv = vec2<f32>(
    apply_address_mode(attribute_uv.x, info.address_mode_u),
    apply_address_mode(attribute_uv.y, info.address_mode_v),
);

let atlas_dimensions = vec2<f32>(textureDimensions(atlas_tex, 0u));
let texel_offset = vec2<f32>(info.pixel_offset);
let texel_size = vec2<f32>(info.size);

// CRITICAL TRANSFORMATION:
let span = max(texel_size - vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 0.0));
let texel_coords = texel_offset + wrapped_uv * span + vec2<f32>(0.5, 0.5);
let uv = texel_coords / atlas_dimensions;
```

### Transformation Breakdown

| Step | Formula | Purpose |
|------|---------|---------|
| 1. Wrap | `wrapped_uv = apply_address_mode(attribute_uv, mode)` | Handle CLAMP/REPEAT/MIRROR |
| 2. Span | `span = max(size - 1.0, 0.0)` | **Why -1.0?** Maps [0,1] to pixel centers within texture |
| 3. Texel Coords | `texel_offset + wrapped_uv * span + 0.5` | Convert from pixel indices to texel centers |
| 4. Normalize | `texel_coords / atlas_dimensions` | Convert to [0,1] atlas space |

**Why the -1.0?**
- A 256×256 texture spans from texel 0 to 255 (not 256)
- In normalized space [0,1], we want to map to centers of first and last texels
- First center: 0.5 / 256 ≈ 0.002
- Last center: 255.5 / 256 ≈ 0.998
- So: texel_index = wrapped_uv * (256 - 1) + 0.5 = wrapped_uv * 255 + 0.5

---

## 3. LOD Calculation in Atlas Space

### Location
`crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`

### Main Function: `compute_texture_lod_atlas_space()` (lines 449-527)

```wgsl
fn compute_texture_lod_atlas_space(
    tex: TextureInfo,
    tri: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    cache: MipCache,
    screen_dims: vec2<f32>
) -> f32
```

### Step-by-Step Analysis

#### Step 1: Compute Local UV Derivatives (lines 461)
```wgsl
var d: UvDerivs = uv_derivs_local(tex, tri, attribute_data_offset, vertex_stride, cache);
```

**Function: `uv_derivs_local()` (lines 324-395)**

Computes derivatives **in local UV space [0,1]**:
- Takes three triangle vertices with their UVs
- Computes screen-space edge positions
- Solves 2×2 linear system: `dUV/dScreen = (UV edge matrix)^-1 * (Screen edge matrix)`

```wgsl
// Screen space edges
let e01_screen = s1 - s0;  // vec2, in screen pixels
let e02_screen = s2 - s0;

// UV space edges  
let e01_uv = uv1 - uv0;    // vec2, in [0,1] space
let e02_uv = uv2 - uv0;

// Solve for dUV/dScreen
let inv_det = 1.0 / (e01_screen.x * e02_screen.y - e01_screen.y * e02_screen.x);
let dudx = (e01_uv.x * inv_s_00 + e02_uv.x * inv_s_10);  // [0,1]/pixel
let dudy = (e01_uv.x * inv_s_01 + e02_uv.x * inv_s_11);
let dvdx = (e01_uv.y * inv_s_00 + e02_uv.y * inv_s_10);  // [0,1]/pixel
let dvdy = (e01_uv.y * inv_s_01 + e02_uv.y * inv_s_11);
```

**Result:** Derivatives are in **local UV space per screen pixel**
- `dudx` = change in local U per screen pixel in X direction
- Range: typically [-2.0, 2.0] after clamping (line 386-391)
- **Units: [0,1] per pixel**

#### Step 2: Convert to Texture Space (lines 490-496)

```wgsl
// Convert local-UV derivatives → texture-space texels/pixel
let dudx_texels = d.dudx * f32(tex.size.x);  // [0,1]/pix * texels → texels/pix
let dudy_texels = d.dudy * f32(tex.size.x);
let dvdx_texels = d.dvdx * f32(tex.size.y);
let dvdy_texels = d.dvdy * f32(tex.size.y);
```

**Transformation:**
- Multiply normalized derivatives by texture dimensions
- Result: **texels per screen pixel**
- Range: e.g., for 256×256 texture: [-512, 512] texels/pixel

#### Step 3: Compute Rho (lines 498-507)

```wgsl
let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);  // X-direction gradient
let rho_y = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);  // Y-direction gradient
let gradient = max(rho_x, rho_y);  // Maximum gradient (isotropic)
```

**What is rho?**
- Standard texture LOD formula: rho = ||∇texture|| (gradient of texture coordinates)
- Computed as max of two directional derivatives
- **Units: texels per screen pixel**

#### Step 4: THE CORRECTION FACTOR (lines 509-518)

```wgsl
// CORRECTION: The derivatives we compute are per-pixel in screen space,
// but the standard formula assumes a specific normalization.
// Empirically, we're getting values ~5-6x too large.
// This suggests we need to scale down by approximately sqrt(2) * 2 ≈ 2.8
// Let's use 0.35 as a correction factor (1/2.8 ≈ 0.35)

let corrected_gradient = gradient * 0.35;
```

#### Step 5: Convert to LOD (lines 520-526)

```wgsl
var lod = log2(max(corrected_gradient, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;
let max_lod = max(atlas.levels_f - 1.0, 0.0);
lod = clamp(lod, 0.0, max_lod);
lod = atlas_clamp_cap(lod, tex, uv_center_local);
```

**Constants:**
```wgsl
const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;  // Sharpen by 0.5 LOD
```

**LOD Formula:**
```
LOD = log2(corrected_gradient) - 0.5
```

---

## 4. Analysis of Critical Issues

### Issue 1: The 0.35 Correction Factor is a Band-Aid

**The Problem:** The comment admits the derivative values are "5-6x too large." The code then applies a 0.35 correction (1/2.85).

**Why This Happens:**

The standard GPU LOD formula is:
```
rho = max(|∂u/∂x|·w, |∂u/∂y|·w, |∂v/∂x|·h, |∂v/∂y|·h)
LOD = log2(max(rho_x, rho_y))
```

Where:
- `(∂u/∂x, ∂v/∂x)` are derivatives in **one screen pixel direction**
- The GPU automatically filters based on 2×2 neighborhoods

**But your code computes:**
1. Full edge vectors (entire triangle edges)
2. Inverts to get per-pixel derivatives
3. These are mathematically correct for the triangle edges
4. But the GPU's LOD selection expects derivatives from **miplevel 0**

**The actual issue:** You're computing correct analytical derivatives, but the LOD formula assumes you're sampling from an **actual texture with bilinear filtering**, where the GPU's gradient operators measure actual sample differences.

### Issue 2: The Atlas Transform Breaks the Math

**The Critical Problem:**

In `compute_texture_lod_atlas_space()`:
1. You compute derivatives in **local [0,1] space** ✓
2. You multiply by texture size to get **texels/pixel** ✓
3. You compute rho and take log2 ✓
4. **MISSING:** You never account for atlas scaling!

When you transform to atlas space:
```wgsl
// From textures.wgsl, _texture_sample_atlas_grad() line 159:
let atlas_scale = span / atlas_dimensions;
let ddx_atlas = ddx_local * atlas_scale;
let ddy_atlas = ddy_local * atlas_scale;
```

The derivatives need this atlas scaling! But `compute_texture_lod_atlas_space()` **doesn't apply it**.

**What this means:**
- Local derivatives: ~256 texels/pixel (for 256×256 texture)
- Atlas derivatives need to be scaled by: `span / atlas_dimensions`
- If your atlas is 4096×4096 and texture is 256×256 at offset (100, 200):
  - `span = 255`
  - `atlas_scale = 255 / 4096 ≈ 0.062`
  - Correct atlas derivative: 256 * 0.062 ≈ 16 texels/pixel in atlas space

**Current code skips this!**

### Issue 3: Mipmap Levels Reference Wrong Space

**From line 488-488:**
```wgsl
if (!atlas.valid) {
    // Fallback using raw UV coordinates
    let du = max(max(abs(uv0.x - uv1.x), abs(uv1.x - uv2.x)), abs(uv2.x - uv0.x));
    let dv = max(max(abs(uv0.y - uv1.y), abs(uv1.y - uv2.y)), abs(uv2.y - uv0.y));
    let rho = max(du * f32(tex.size.x), dv * f32(tex.size.y));
    var lod_fb = log2(max(rho, 1e-6));
    lod_fb = clamp(lod_fb + MIPMAP_GLOBAL_LOD_BIAS, 0.0, 0.0);  // BUG: clamped to 0!
    return atlas_clamp_cap(lod_fb, tex, uv_center_local);
}
```

**Problem:** Line 486 clamps LOD to [0.0, 0.0] — this should be [0.0, max_lod]!

### Issue 4: Constants Don't Match Reality

**Constants (lines 5-8):**
```wgsl
const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;
const MIPMAP_CLAMP_EPSILON   : f32 = 1e-4;
const MIPMAP_MIN_DET         : f32 = 1e-6;
const MIPMAP_ATLAS_PADDING   : f32 = 8.0; // texels of content padding per sub-rect
```

**Issues:**
1. `MIPMAP_GLOBAL_LOD_BIAS = -0.5` sharpens textures by 0.5 LOD (2√2 higher resolution)
   - This might mask the 0.35 correction inadequacy
   - Combined effect: 0.35 * 2^0.5 ≈ 0.495 — almost unity!

2. `MIPMAP_ATLAS_PADDING = 8.0` — but derivatives don't account for padding border effects

---

## 5. Derivative Space Analysis

### Question: Are derivatives in local [0,1] or pixel space?

**Answer: Local [0,1] per screen pixel**

**Evidence from `uv_derivs_local()` (lines 324-395):**

```wgsl
let e01_uv = uv1 - uv0;    // [0, 1] normalized coordinates
// ... matrix inversion ...
let dudx = e01_uv.x * inv_s_00 + e02_uv.x * inv_s_10;
```

Output: `dudx` has units of `[0,1] / screen_pixel`

**Verification:**
- `e01_uv` is in [0,1] space (UV coordinates)
- `inv_s` matrix has units of `1/screen_pixel`
- Result: `[0,1] * (1/screen_pixel) = [0,1] / screen_pixel`

**So: derivatives ARE in the right space for LOD calculation, BUT the LOD formula assumes something different.**

---

## 6. Relationship Between Texture Size, Atlas Size, and Mip Levels

### Mip Level Computation

**Atlas mipmaps:** Created for entire atlas
```wgsl
let lvls = f32(textureNumLevels(atlas_tex_{{ i }}));  // For entire atlas
```

**Texture within atlas:** 
- Occupies rect [offset, offset+size] in mip 0 of atlas
- To access LOD L: divide both size and offset by 2^L
- Texture at LOD L occupies: [(offset/2^L), (offset+size)/2^L]

**Current issue:** The LOD calculation computes LOD as if the texture is standalone, but then directly uses that LOD on the atlas texture. This works numerically ONLY if:

```
texture_lod = atlas_lod  (approximately)
```

This is TRUE for LOD calculation based on **atlas-space rho**, not texture-space rho!

### The Real Fix

**Current (incorrect):**
```wgsl
// Compute in texture space (normalized [0,1])
let dudx_texels = d.dudx * f32(tex.size.x);
let lod = log2(dudx_texels);  // Uses texture size
// Apply to atlas with that LOD
```

**Should be:**
```wgsl
// Compute in texture space (for reference)
let dudx_texels = d.dudx * f32(tex.size.x);

// Scale to atlas space
let span = max(tex.size - vec2<u32>(1), vec2<u32>(0));
let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0u));
let scale_factor = f32(span.x) / atlas_dims.x;  // For X direction

// Compute rho in atlas space
let dudx_atlas_texels = dudx_texels * scale_factor;

// LOD based on atlas-space rho
let lod = log2(dudx_atlas_texels);
```

---

## 7. Specific Inconsistencies and Bugs

| # | Issue | Location | Severity | Fix |
|---|-------|----------|----------|-----|
| 1 | Mipmap level clamp bug | line 486 | HIGH | Change `clamp(..., 0.0, 0.0)` to `clamp(..., 0.0, max_lod)` |
| 2 | No atlas scaling in LOD calc | line 493-520 | HIGH | Apply atlas transform to derivatives before LOD |
| 3 | 0.35 correction is a band-aid | line 518 | MEDIUM | Root cause: mixing texture-space and atlas-space math |
| 4 | LOD bias may be masking issues | line 5 | MEDIUM | Verify with correct atlas scaling first |
| 5 | No anisotropic LOD selection | line 507 | LOW | Use both rho_x and rho_y for anisotropic filtering support |

---

## 8. Correct LOD Formula for Atlas-Based Textures

### Standard GPU LOD Formula
```
rho = max(|(du/dx)·w|, |(du/dy)·w|, |(dv/dx)·h|, |(dv/dy)·h|)
LOD = max(0, log2(rho) + bias)
```

### Corrected Formula for Your System

**Given:**
- Local UV derivatives: `dudx`, `dudy`, `dvdx`, `dvdy` in [0,1]/pixel
- Texture size: `tex.size` in pixels
- Atlas dimensions: `atlas_dims`
- Texture position in atlas: `tex.pixel_offset`

**Compute:**
```wgsl
// 1. Convert to texture-space texels/pixel
let dudx_texels = d.dudx * f32(tex.size.x);
let dvdx_texels = d.dvdx * f32(tex.size.y);
let dudy_texels = d.dudy * f32(tex.size.x);
let dvdy_texels = d.dvdy * f32(tex.size.y);

// 2. Compute texture-space rho
let rho_x_texels = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let rho_y_texels = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);
let rho_texels = max(rho_x_texels, rho_y_texels);

// 3. Account for atlas packing
let span = vec2<f32>(max(tex.size - vec2<u32>(1), vec2<u32>(0)));
let atlas_dims = vec2<f32>(textureDimensions(atlas, 0u));
// Average scale factor (approximate for non-square textures)
let atlas_scale = (span.x / atlas_dims.x + span.y / atlas_dims.y) * 0.5;

// 4. Convert to atlas-space texels/pixel (these match mipmap levels)
let rho_atlas = rho_texels * atlas_scale;

// 5. Clamp to avoid extreme values
let clamped_rho = clamp(rho_atlas, 1e-6, 1e3);

// 6. Select LOD
let lod = log2(clamped_rho) + MIPMAP_GLOBAL_LOD_BIAS;
```

---

## 9. Summary Table

### Derivative Computation
| Stage | Space | Units | Range |
|-------|-------|-------|-------|
| Input | Local UV | [0,1] | Various |
| After `uv_derivs_local()` | Local UV/pixel | [0,1]/px | [-2, 2] |
| After multiply by size | Texture-space | texels/px | [-512, 512] |
| After atlas scaling | **Atlas-space** | atlas-texels/px | Varies with position |
| After log2 | **LOD** | mip levels | [0, num_levels] |

### Current Issues
1. **Missing atlas scaling** → LOD computed in wrong space
2. **0.35 correction** → Masks real problem, empirical band-aid
3. **Bias of -0.5** → May compensate for scaling errors
4. **Clamp bug** → Fallback path broken
5. **No anisotropic support** → Max of gradients hides directional quality differences

---

## 10. Recommended Fix Strategy

**Phase 1: Fix obvious bug**
- Fix line 486 clamp issue

**Phase 2: Implement correct atlas scaling**
- Modify `compute_texture_lod_atlas_space()` to apply atlas transform
- Remove or significantly reduce 0.35 correction
- Adjust bias as needed after testing

**Phase 3: Validate**
- Test on various texture sizes and atlas positions
- Compare LOD selections with GPU-computed gradients
- Measure aliasing/blur artifacts

**Phase 4: Optimize (optional)**
- Consider pre-computing atlas scales as uniforms
- Add anisotropic LOD selection
- Cache mip computation for repeated lookups

