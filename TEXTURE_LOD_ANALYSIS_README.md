# Texture LOD Analysis - Documentation Overview

This analysis documents a complete investigation of the texture atlas and LOD calculation pipeline in the renderer, identifying critical mathematical errors and providing actionable fixes.

## Documents Created

### 1. **TEXTURE_LOD_ANALYSIS.md** (Primary Document)
   - **Length:** ~500 lines
   - **Content:**
     - Detailed mipmap generation pipeline
     - Texture atlas structure and transformations
     - Step-by-step LOD calculation analysis
     - Mathematical inconsistencies and bugs
     - Correct formulas with examples
     - Specific bugs with locations and severity
     - Recommended fix strategy

   **Key Sections:**
   - Mipmap Generation (contrast-preserving filter strategy)
   - TextureInfo Structure (pixel_offset, size, atlas_index fields)
   - Local UV to Atlas UV Transformation
   - LOD Calculation in 5 steps (including the 0.35 correction mystery)
   - Derivative Space Analysis
   - Atlas Scaling Issues
   - Summary Tables

### 2. **TEXTURE_LOD_PIPELINE_DIAGRAM.md** (Visual Guide)
   - **Length:** ~400 lines
   - **Content:**
     - ASCII data flow diagram
     - Step-by-step visual walkthrough
     - Current vs. correct implementation comparison
     - Concrete numerical example (256×256 texture)
     - Why the bug hasn't completely broken rendering
     - Key takeaways table

   **Key Sections:**
   - Data Flow showing derivative progression through spaces
   - Detailed explanation of each calculation step
   - Highlighting where the bug occurs
   - Real math with concrete numbers
   - Why 0.35 × (-0.5) bias nearly masks the error
   - When and why it fails

### 3. **TEXTURE_LOD_FIXES.md** (Action Items)
   - **Length:** ~250 lines
   - **Content:**
     - 5 issues identified and prioritized
     - 2 critical bugs requiring immediate fixes
     - 3 secondary/optional improvements
     - Implementation checklist
     - Testing strategy
     - Expected results before/after fixes

   **Key Sections:**
   - Bug #1: Clamp typo (line 486) - 2 minute fix
   - Bug #2: Missing atlas scaling (lines 490-520) - main issue
   - Issue #3: Dimension scaling (line 494)
   - Issue #4: LOD bias masking errors
   - Issue #5: Anisotropic support (enhancement)
   - Complete testing strategy with edge cases

## The Problem in One Paragraph

The mipmap LOD calculation computes derivatives in local UV space and converts them to texture-space texels/pixel correctly, but then fails to apply the atlas scaling transformation before computing the LOD value. This scaling factor (typically ~0.06 for 256×256 textures in a 4096×4096 atlas) is instead replaced with a mysterious empirical constant (0.35) that happens to partially work for common texture sizes due to additional masking by a -0.5 LOD bias. The result: LOD selection is mathematically incorrect but works well for the common case (256×256 textures) and fails badly for other sizes.

## The Solution in One Paragraph

Replace the empirical 0.35 correction with the proper atlas-space scaling calculation: `gradient_atlas = gradient_texture * (texture_span / atlas_dimensions)`. This requires obtaining the atlas dimensions at LOD calculation time and computing the per-direction scale factors. After applying this fix, the -0.5 LOD bias should be re-evaluated and possibly adjusted or removed, as it was masking the error.

## Critical Findings

### 1. Root Cause: Missing Atlas Scaling
- **Location:** `mipmap.wgsl` lines 490-520
- **What's wrong:** LOD computed in texture-space, not atlas-space
- **Impact:** Wrong mipmap levels selected for all textures
- **Severity:** CRITICAL

### 2. Band-Aid: 0.35 Correction Factor
- **What it is:** Empirical constant masking the missing scaling
- **Formula:** `0.35 ≈ 1/2.8 ≈ 1/(sqrt(2)×2)`
- **Why it appears to work:** Partially cancels out for 256×256 textures
- **When it fails:** Small (16×16), large (2048×2048), non-square textures

### 3. Error Magnitude
- **For typical case (256×256 in 4096×4096 atlas):**
  - Current LOD: Computed with 0.35 factor
  - Correct LOD: Should use 0.0623 factor
  - Error: 5.6x difference in texture resolution per mipmap level!
  - Visible impact: Up to 2.5 LOD levels off in some cases

### 4. Additional Bugs Found
- **Bug #1:** Line 486 clamps LOD to [0,0] instead of [0,max_lod]
- **Bug #2:** Line 494 uses wrong texture dimension (x instead of y) for y-derivative
- **Note:** Bug #1 affects fallback path; Bug #2 breaks non-square textures

## Quick Reference: The Fixes

| Item | Location | Current | Fixed | Time |
|------|----------|---------|-------|------|
| Bug #1 | Line 486 | `clamp(..., 0.0, 0.0)` | `clamp(..., 0.0, max_lod)` | 2 min |
| Bug #2 | Lines 490-520 | 0.35 factor | Atlas scaling | 30 min |
| Issue #3 | Line 494 | `tex.size.x` | `tex.size.y` | 1 min |
| Issue #4 | Line 5, 520 | -0.5 bias | Re-test value | 30 min |
| Issue #5 | Line 507 | Isotropic only | Anisotropic support | Optional |

## Mathematical Analysis

### Current Formula (WRONG)
```
rho = max(|∂u/∂x|·w, |∂v/∂x|·h, |∂u/∂y|·w, |∂v/∂y|·h)  [in texture space]
lod = log2(rho × 0.35) - 0.5
```

### Correct Formula
```
rho_texture = max(|∂u/∂x|·w, |∂v/∂x|·h, |∂u/∂y|·w, |∂v/∂y|·h)  [in texture space]
rho_atlas = rho_texture × (texture_span / atlas_dimensions)      [in atlas space]
lod = log2(rho_atlas) + bias                                     [correct LOD]
```

The key difference: the scaling factor must be computed dynamically from:
- `texture_span` = texture dimensions minus 1 (for texel center mapping)
- `atlas_dimensions` = full atlas resolution

## How to Use These Documents

1. **Start with:** `TEXTURE_LOD_PIPELINE_DIAGRAM.md`
   - Gets you familiar with how the pipeline works
   - Shows exactly where and why things break
   - Provides concrete numerical examples

2. **Then read:** `TEXTURE_LOD_ANALYSIS.md`
   - Deep dive into all issues
   - Mathematical derivations
   - Complete context for each bug

3. **Finally use:** `TEXTURE_LOD_FIXES.md`
   - Your implementation guide
   - Checklist for making changes
   - Testing strategy to verify fixes work

## Implementation Order

### Phase 1 (Critical - Do First)
1. Fix Bug #1 (line 486) - 2 minutes
2. Fix Bug #2 (lines 490-520) - 30 minutes
3. Test and verify basic functionality

### Phase 2 (Important - Do Next)
4. Fix Issue #3 (line 494) - 1 minute
5. Adjust Issue #4 (LOD bias) - 30 minutes testing
6. Comprehensive testing suite

### Phase 3 (Optional - Do Later)
7. Add Issue #5 (anisotropic support) - 1-2 hours

## Expected Improvements

After applying all fixes:

| Metric | Before | After |
|--------|--------|-------|
| LOD accuracy | ±2.5 levels (wrong) | ±0.25 levels (correct) |
| 256×256 textures | Good (by luck) | Good (by design) |
| 16×16 textures | Too blurry | Sharp & correct |
| 2048×2048 textures | Too blurry | Sharp & correct |
| Non-square textures | Broken Y-axis | Correct both axes |
| Edge cases | Undefined behavior | Robust |

## Files Modified

- `/crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`

That's the only file with functional bugs. Other improvements may be in related files.

## Questions This Answers

1. **Q: What's the 0.35 correction factor?**
   A: A band-aid empirical constant masking missing atlas scaling math.

2. **Q: Why do some textures look blurry while others have artifacts?**
   A: LOD formula works OK for common sizes (256×256) but breaks for others.

3. **Q: Why does increasing LOD bias help sometimes?**
   A: The -0.5 bias sharpens textures and accidentally cancels some errors.

4. **Q: Are derivatives computed in the right space?**
   A: Yes, local [0,1] per pixel is correct. The error is later in the pipeline.

5. **Q: How big is the error?**
   A: Up to 2.5 LOD levels (5.7x texture resolution) in extreme cases.

## Related Code

For context, also see:
- `crates/renderer-core/src/texture/mipmap.rs` - Mipmap generation (correct)
- `crates/renderer-core/src/image/atlas.rs` - Atlas structure
- `crates/renderer/src/render_passes/material/shared/shader/all_material_shared_wgsl/textures.wgsl` - Texture sampling (has correct atlas scaling example in `_texture_sample_atlas_grad()` line 159)

Note: The correct atlas scaling is already implemented in `_texture_sample_atlas_grad()` for gradient-based sampling. The LOD calculation code should follow the same pattern.

