# Texture LOD Calculation Pipeline - Visual Walkthrough

## 1. Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         INPUT: Fragment Shader                               │
│  - Triangle vertices with UVs                                               │
│  - Screen-space positions (from geometry pass)                              │
│  - TextureInfo (size, offset, atlas_index, address_modes)                   │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                  STEP 1: Compute UV Derivatives                              │
│                     uv_derivs_local() [lines 324-395]                       │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Input:  tri.xyz = [vertex0, vertex1, vertex2] indices              │   │
│  │         uv0, uv1, uv2 = UV coordinates [0,1]                       │   │
│  │         s0, s1, s2 = screen-space positions [pixels]               │   │
│  │                                                                       │   │
│  │ Process:  e01_uv = uv1 - uv0         // UV edges [0,1]            │   │
│  │           e02_uv = uv2 - uv0                                       │   │
│  │           e01_screen = s1 - s0       // Screen edges [pixels]      │   │
│  │           e02_screen = s2 - s0                                     │   │
│  │                                                                       │   │
│  │           Solve: dUV/dScreen = (UV matrix)^-1 × (Screen matrix)    │   │
│  │                                                                       │   │
│  │           // 2×2 matrix inversion                                   │   │
│  │           det = e01_screen.x × e02_screen.y - e01_screen.y × e02_screen.x  │   │
│  │           inv_det = 1.0 / det                                      │   │
│  │           // Inverse components...                                 │   │
│  │           dudx = e01_uv.x * inv_s_00 + e02_uv.x * inv_s_10       │   │
│  │           dvdx = e01_uv.y * inv_s_00 + e02_uv.y * inv_s_10       │   │
│  │           // ...etc for dy                                          │   │
│  │                                                                       │   │
│  │ Output: dudx, dudy, dvdx, dvdy   [units: [0,1] per screen pixel]  │   │
│  │         Range after clamp: [-2.0, 2.0]                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  SPACE: Local UV [0,1]        UNITS: [0,1] / screen pixel                 │
│  CORRECTNESS: ✓ CORRECT                                                    │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│           STEP 2: Convert to Texture-Space Texels/Pixel                     │
│                    [lines 490-496]                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ dudx_texels = dudx * tex.size.x    // [0,1]/px × texels → texels/px  │   │
│  │ dvdx_texels = dvdx * tex.size.y                                     │   │
│  │ dudy_texels = dudy * tex.size.x                                     │   │
│  │ dvdy_texels = dvdy * tex.size.y                                     │   │
│  │                                                                       │   │
│  │ Example (256×256 texture):                                          │   │
│  │   dudx = 0.5 [0,1]/px  →  dudx_texels = 0.5 × 256 = 128 texels/px │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  SPACE: Texture-space      UNITS: texels / screen pixel                   │
│  RANGE: ≈ [-512, 512] for 256×256 texture                                │
│  CORRECTNESS: ✓ CORRECT                                                    │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│            STEP 3: Compute Rho (Texture Gradient)                           │
│                    [lines 498-507]                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ rho_x = sqrt(dudx_texels² + dvdx_texels²)    // X-direction gradient│   │
│  │ rho_y = sqrt(dudy_texels² + dvdy_texels²)    // Y-direction gradient│   │
│  │ gradient = max(rho_x, rho_y)                  // Isotropic (max)     │   │
│  │                                                                       │   │
│  │ Conceptually: Maximum rate of change in texture space per pixel     │   │
│  │               If gradient=128, texture changes by 128 texels/pixel  │   │
│  │                                                                       │   │
│  │ ISSUE: This is computed in TEXTURE SPACE, but...                   │   │
│  │        We'll apply the result to the ATLAS                          │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  SPACE: Texture-space (NOT atlas-space!)                                   │
│  UNITS: texels / screen pixel                                             │
│  CORRECTNESS: ✗ WRONG SPACE - THIS IS THE BUG!                           │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│          STEP 4: Apply Mysterious 0.35 Correction Factor                    │
│                    [lines 509-518]                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ // CORRECTION: The derivatives we compute are per-pixel in screen   │   │
│  │ // space, but the standard formula assumes a specific normalization.│   │
│  │ // Empirically, we're getting values ~5-6x too large.              │   │
│  │                                                                       │   │
│  │ corrected_gradient = gradient * 0.35                                │   │
│  │                                                                       │   │
│  │ QUESTION: Why 0.35?                                                 │   │
│  │ ANSWER (from comment): ~1/2.8 ≈ 0.35                               │   │
│  │          sqrt(2) × 2 ≈ 2.8, so 1/2.8 ≈ 0.355                       │   │
│  │                                                                       │   │
│  │ REALITY: This is a band-aid masking the missing atlas scaling!     │   │
│  │          Should be: gradient * (texture_span / atlas_dims)         │   │
│  │                                                                       │   │
│  │ Example breakdown for 256×256 texture in 4096×4096 atlas:          │   │
│  │   texture_span = 255                                                │   │
│  │   atlas_dims = 4096                                                │   │
│  │   correct_scale = 255 / 4096 ≈ 0.062                              │   │
│  │   applied_scale = 0.35                                             │   │
│  │   ERROR: Using 0.35 instead of 0.062 → OFF BY 5.6x!               │   │
│  │                                                                       │   │
│  │ ADDITIONAL MASKING:                                                 │   │
│  │   LOD bias = -0.5  (sharpens by 2^0.5 ≈ 1.41)                     │   │
│  │   Combined effect: 0.35 × 2^0.5 ≈ 0.495 ≈ 0.5 (almost unity!)     │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  CORRECTNESS: ✗✗ HIGHLY INCORRECT - PRIMARY BUG                           │
│  SEVERITY: HIGH - Band-aid hiding fundamental atlas scaling issue           │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│             STEP 5: Convert to LOD via log2                                 │
│                    [lines 520-526]                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ lod = log2(max(corrected_gradient, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS │   │
│  │                                          + (-0.5)                   │   │
│  │                                                                       │   │
│  │ // Clamp to valid LOD range                                         │   │
│  │ max_lod = textureNumLevels(atlas) - 1.0                            │   │
│  │ lod = clamp(lod, 0.0, max_lod)                                     │   │
│  │                                                                       │   │
│  │ // Clamp for edge cases (CLAMP_TO_EDGE addressing)                 │   │
│  │ lod = atlas_clamp_cap(lod, tex, uv_center_local)                   │   │
│  │                                                                       │   │
│  │ Example:                                                             │   │
│  │   corrected_gradient = 0.35 * 128 = 44.8                           │   │
│  │   lod = log2(44.8) - 0.5                                           │   │
│  │       = 5.49 - 0.5 = 4.99 ≈ 5                                      │   │
│  │   Result: Will sample from mipmap level 5                          │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  OUTPUT: lod  [units: mipmap level, range: 0.0 to max_lod]                │
│  CORRECTNESS: ✗ INCORRECT LOD SPACE DUE TO EARLIER ERRORS                 │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    OUTPUT: Apply LOD to Atlas Sampling                      │
│                                                                              │
│  Result is passed to texture_sample_atlas() [textures.wgsl line 66]       │
│  Which calls: textureSampleLevel(atlas_tex, sampler, uv, layer, lod)      │
│                                                                              │
│  THE LOD IS INTERPRETED IN ATLAS SPACE!                                   │
│  But it was computed in TEXTURE SPACE with empirical correction!           │
│                                                                              │
│  This explains why sampling "works" despite being mathematically wrong:    │
│  The 0.35 factor + (-0.5) bias partially compensate for the missing       │
│  atlas scaling in many common cases (256×256 textures at typical atlas    │
│  positions), but breaks down for:                                          │
│    - Very small textures (16×16)                                           │
│    - Very large textures (2048×2048)                                       │
│    - Extreme atlas positions (deeply packed)                               │
│    - Non-square textures (256×512)                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 2. Comparison: Current vs. Correct

### Current Implementation

```wgsl
// Step 1: Get derivatives in local [0,1] space
var d = uv_derivs_local(...);  // dudx, dvdx in [0,1]/pixel

// Step 2: Convert to texture space
let dudx_texels = d.dudx * f32(tex.size.x);  // texels/pixel

// Step 3: Compute gradient
let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let gradient = max(rho_x, rho_y);

// Step 4: MYSTERY CORRECTION ← BUG IS HERE
let corrected_gradient = gradient * 0.35;  // 0.35 = 1/2.8

// Step 5: LOD
let lod = log2(corrected_gradient) - 0.5;
```

**Space progression:**
```
[0,1]/pixel → texture-space → ??? 
            [missing: atlas scaling]
```

### Correct Implementation (Proposed)

```wgsl
// Step 1: Get derivatives in local [0,1] space
var d = uv_derivs_local(...);  // dudx, dvdx in [0,1]/pixel

// Step 2: Convert to texture space
let dudx_texels = d.dudx * f32(tex.size.x);  // texels/pixel in texture
let dvdx_texels = d.dvdx * f32(tex.size.y);
let dudy_texels = d.dudy * f32(tex.size.x);
let dvdy_texels = d.dvdy * f32(tex.size.y);

// Step 3: Compute gradient in texture space
let rho_x_texels = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let rho_y_texels = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);
let rho_texels = max(rho_x_texels, rho_y_texels);

// Step 4: APPLY ATLAS SCALING ← FIX!
let span = vec2<f32>(max(tex.size - vec2<u32>(1), vec2<u32>(0)));
let atlas_dims = vec2<f32>(textureDimensions(atlas, 0u));
// More sophisticated: scale per-direction instead of averaging
let scale_x = span.x / atlas_dims.x;
let scale_y = span.y / atlas_dims.y;

// Convert to atlas space (what matters for LOD!)
let rho_atlas = max(rho_x_texels * scale_x, rho_y_texels * scale_y);

// Step 5: LOD (now in correct space!)
let lod = log2(max(rho_atlas, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;
```

**Space progression:**
```
[0,1]/pixel → texture-space → atlas-space → LOD
           ✓ correct         ✓ correct   ✓ correct
```

## 3. Concrete Example

### Scenario
- **Texture:** 256×256 pixels, at offset (1024, 512) in atlas
- **Atlas:** 4096×4096 pixels with 10 mip levels
- **Surface:** Nearly perpendicular to camera, pixel projects to ~0.5×0.5 texture area

### Calculation

**Local derivatives (from triangle edges):**
```
dudx = 0.5  [0,1] per pixel   (half the texture width per screen pixel)
dvdx = 0.1  [0,1] per pixel
dudy = 0.05 [0,1] per pixel
dvdy = 0.3  [0,1] per pixel
```

**Step 1: Convert to texture space**
```
dudx_texels = 0.5 × 256 = 128 texels/pixel in X
dvdx_texels = 0.1 × 256 = 25.6 texels/pixel in X
dudy_texels = 0.05 × 256 = 12.8 texels/pixel in Y
dvdy_texels = 0.3 × 256 = 76.8 texels/pixel in Y
```

**Step 2: Compute rho in texture space**
```
rho_x = sqrt(128² + 25.6²) = sqrt(16384 + 655.36) ≈ 130
rho_y = sqrt(12.8² + 76.8²) = sqrt(163.84 + 5898.24) ≈ 76.9
rho_texels = max(130, 76.9) = 130 texels/pixel
```

**Step 3 CURRENT (WRONG): Apply 0.35 correction**
```
corrected_gradient = 130 × 0.35 = 45.5
lod = log2(45.5) - 0.5 = 5.51 - 0.5 = 5.01
→ Sample from mipmap level 5
```

**Step 3 CORRECT: Apply atlas scaling**
```
span = 255 (256 - 1)
scale_x = 255 / 4096 ≈ 0.0623
scale_y = 255 / 4096 ≈ 0.0623

// Per-direction scaling:
rho_x_atlas = 130 × 0.0623 ≈ 8.1 texels/pixel (atlas-relative)
rho_y_atlas = 76.9 × 0.0623 ≈ 4.8 texels/pixel (atlas-relative)
rho_atlas = max(8.1, 4.8) = 8.1

lod = log2(8.1) - 0.5 = 3.02 - 0.5 = 2.52
→ Sample from mipmap level 2 (much sharper!)
```

**Comparison:**
```
Current (WRONG):  LOD = 5.01  → mip level 5  → 256/32 = 8×8 texture area → BLURRY
Correct:          LOD = 2.52  → mip level 2  → 256/4 = 64×64 texture area → SHARP

Error magnitude: 2.5 LOD levels = 2^2.5 ≈ 5.7x difference in texture resolution!
```

## 4. Why This Hasn't Completely Broken Rendering

### The Masking Effect

The bug is **partially hidden** by:

1. **The 0.35 factor:** Random empirical value that's close-ish for certain cases
2. **The -0.5 LOD bias:** Sharpens by 2^0.5 ≈ 1.41
3. **Combined effect:** 0.35 × 1.41 ≈ 0.49 ≈ 0.5 → Nearly unity for many common textures
4. **Common texture size:** 256×256 textures at typical atlas positions
   - Factors partially cancel for this specific case
   - Other texture sizes show obvious artifacts

### When It Fails

- 16×16 textures → 0.35 factor is way too high → excessive blur
- 2048×2048 textures → 0.35 factor is too low → aliasing/sparkle
- Extreme atlas positions → scale factor varies wildly
- Non-square textures (e.g., 512×256) → different X vs Y errors

## 5. Key Takeaways

| Aspect | Current | Correct |
|--------|---------|---------|
| **Derivative Computation** | ✓ Correct | ✓ Same |
| **Atlas Scaling Applied** | ✗ No | ✓ Yes |
| **Correction Factor** | 0.35 (empirical band-aid) | Proper atlas scaling |
| **LOD Bias** | -0.5 (masks errors) | -0.5 (or adjusted after fix) |
| **Works for** | 256×256 at typical positions | All texture sizes, all positions |
| **Breaks for** | Small/large textures, extreme positions | (Should be robust) |

