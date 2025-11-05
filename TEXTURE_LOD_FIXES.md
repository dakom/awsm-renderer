# Texture LOD Calculation - Fixes to Implement

## Summary

The mipmap selection has **5 distinct issues**, with **2 critical bugs** that must be fixed:

### Critical Bugs (HIGH PRIORITY)

#### Bug #1: Clamp Typo in Fallback Code (Line 486)

**Location:** `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`, line 486

**Current (WRONG):**
```wgsl
lod_fb = clamp(lod_fb + MIPMAP_GLOBAL_LOD_BIAS, 0.0, 0.0);  // Clamps to [0, 0]!
```

**Fixed:**
```wgsl
let max_lod = max(atlas.levels_f - 1.0, 0.0);
lod_fb = clamp(lod_fb + MIPMAP_GLOBAL_LOD_BIAS, 0.0, max_lod);
```

**Impact:** This completely breaks the fallback path when `atlas.valid` is false. The LOD is clamped to exactly 0, forcing mipmap level 0 (highest detail). This path is hit when:
- Atlas info can't be retrieved
- Multiple atlas indices are accessed
- Edge cases in shader compilation

**Severity:** HIGH - Causes crashes or incorrect LOD in edge cases

---

#### Bug #2: Missing Atlas Scaling in LOD Calculation (Lines 490-520)

**Location:** `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`, lines 490-520

**Problem:** The LOD is computed in **texture-space** but applied to the **atlas**, which requires a scaling factor that's completely missing.

**Current (WRONG):**
```wgsl
// Line 490-496: Convert to texture space
let dudx_texels = d.dudx * f32(tex.size.x);
let dudy_texels = d.dudy * f32(tex.size.x);  // BUG: Should use tex.size.y!
let dvdx_texels = d.dvdx * f32(tex.size.y);
let dvdy_texels = d.dvdy * f32(tex.size.y);

// Line 498-507: Compute gradient in texture space
let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let rho_y = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);
let gradient = max(rho_x, rho_y);

// Line 509-518: MYSTERY CORRECTION (band-aid)
let corrected_gradient = gradient * 0.35;  // 0.35 = missing atlas scaling!

// Line 520-526: LOD (now interpreted as atlas LOD)
var lod = log2(max(corrected_gradient, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;
```

**Fixed:**
```wgsl
// Step 1: Convert to texture-space derivatives (same as before)
let dudx_texels = d.dudx * f32(tex.size.x);
let dudy_texels = d.dudy * f32(tex.size.x);  // Also fix this
let dvdx_texels = d.dvdx * f32(tex.size.y);
let dvdy_texels = d.dvdy * f32(tex.size.y);

// Step 2: Compute gradient in texture space
let rho_x_texels = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let rho_y_texels = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);

// Step 3: APPLY ATLAS SCALING (this is the fix!)
let span = vec2<f32>(max(tex.size - vec2<u32>(1), vec2<u32>(0)));
let atlas_dims = vec2<f32>(textureDimensions(atlas, 0u));

// Per-direction scaling for accuracy
let scale_x = span.x / atlas_dims.x;
let scale_y = span.y / atlas_dims.y;

// Convert to atlas-space gradients
let rho_x_atlas = rho_x_texels * scale_x;
let rho_y_atlas = rho_y_texels * scale_y;
let gradient_atlas = max(rho_x_atlas, rho_y_atlas);

// Step 4: LOD (now in correct atlas space!)
var lod = log2(max(gradient_atlas, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;

// Step 5: Clamp to valid range
let max_lod = max(atlas.levels_f - 1.0, 0.0);
lod = clamp(lod, 0.0, max_lod);

// Step 6: Handle edge cases
lod = atlas_clamp_cap(lod, tex, uv_center_local);
```

**Impact:** This is the root cause of mipmap selection issues:
- Textures appear too blurry or too sharp depending on size/position
- The 0.35 correction factor is an empirical band-aid that only works for 256×256 textures at typical positions
- Extreme LOD selections on very small or very large textures
- Aliasing artifacts on undersampled textures

**Severity:** CRITICAL - Fundamental algorithmic error

---

### Secondary Issues (MEDIUM PRIORITY)

#### Issue #3: Inconsistent Texture Dimension Scaling (Line 494)

**Location:** Line 494

**Current (WRONG):**
```wgsl
let dudy_texels = d.dudy * f32(tex.size.x);  // Uses X size for Y derivative!
```

**Fixed:**
```wgsl
let dudy_texels = d.dudy * f32(tex.size.y);  // Use Y size for Y derivative
```

**Impact:** Non-square textures (256×512, 512×256) compute Y derivatives incorrectly. This compounds with Bug #2.

---

#### Issue #4: LOD Bias Masking Errors (Line 5, 520)

**Location:** Lines 5 and 520

**Current:**
```wgsl
const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;  // Sharpens by 2^0.5 ≈ 1.41x
```

**Analysis:**
- The -0.5 bias sharpens textures (selects higher detail)
- Combined with 0.35 correction: 0.35 × 2^0.5 ≈ 0.495 ≈ 0.5
- This nearly cancels out errors for specific texture sizes
- Acts as a mask that hides Bug #2

**After applying Bug #2 fix:**
- Verify the -0.5 bias is still appropriate
- May need to reduce it to -0.25 or remove it entirely
- This should be determined by visual testing on diverse textures

**Recommendation:** After fixing Bugs #1 and #2, test with:
```wgsl
const MIPMAP_GLOBAL_LOD_BIAS : f32 = 0.0;  // Test without bias first
```

Then tune to desired sharpness level.

---

#### Issue #5: No Anisotropic LOD Support (Line 507)

**Location:** Line 507

**Current:**
```wgsl
let gradient = max(rho_x, rho_y);  // Isotropic (takes maximum)
```

**Enhancement (Optional):**
```wgsl
// Could support anisotropic filtering by using both gradients
// For now, max is fine for isotropic sampling
let gradient = max(rho_x, rho_y);

// Future: for anisotropic support, compute different LODs per direction
let lod_x = log2(max(rho_x, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;
let lod_y = log2(max(rho_y, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;
// Use anisotropic sampler config based on lod_x vs lod_y ratio
```

**Impact:** Currently isotropic filtering only. This is acceptable but limits quality on oblique surfaces.

**Severity:** LOW - Enhancement, not a bug

---

## Implementation Checklist

### Phase 1: Fix Critical Bugs (Do First)

- [ ] Fix Bug #1: Line 486 clamp
  - Change: `clamp(..., 0.0, 0.0)` → `clamp(..., 0.0, max_lod)`
  - Time: 2 minutes
  - Risk: None (fixes obviously wrong code)

- [ ] Fix Bug #2: Add atlas scaling (Lines 490-527)
  - Change: Replace 0.35 correction with proper atlas scaling
  - Time: 15-30 minutes
  - Risk: Medium (requires careful testing)

### Phase 2: Fix Secondary Issues (Do Next)

- [ ] Fix Issue #3: Line 494 dimension
  - Change: `tex.size.x` → `tex.size.y`
  - Time: 1 minute
  - Risk: None

- [ ] Adjust Issue #4: LOD bias
  - Test with different values: -0.5, -0.25, 0.0
  - Time: 30 minutes (testing)
  - Risk: None (visual testing only)

### Phase 3: Enhancement (Do Later)

- [ ] Add Issue #5: Anisotropic support
  - Time: 1-2 hours
  - Risk: Low (isolated enhancement)

---

## Testing Strategy

### After Fixing Bugs #1 and #2

1. **Visual Inspection**
   - Load various models with different texture sizes
   - Check for:
     - Excessive blur → LOD too high
     - Aliasing/sparkle → LOD too low
     - Weird transitions → discontinuities in LOD space

2. **Specific Test Cases**
   ```
   a) 16×16 texture in 4096×4096 atlas
      - Should be sharper (was overly blurred)
   
   b) 2048×2048 texture in 4096×4096 atlas
      - Should be sharper (was overly blurred)
   
   c) 256×512 non-square texture
      - Should look consistent with 256×256
      - (Was broken due to Issue #3)
   
   d) Texture at extreme atlas offset (near corner)
      - Should work correctly at all positions
   ```

3. **Edge Case Testing**
   ```
   e) Multiple adjacent textures in atlas
      - Check for bleeding/artifacts at boundaries
   
   f) Repeated textures (REPEAT address mode)
      - LOD should be consistent across repeats
   
   g) Clamped textures (CLAMP_TO_EDGE address mode)
      - Padding borders should be handled correctly
   ```

4. **LOD Value Verification**
   - Add debug output for LOD values
   - Compare with GPU-computed derivatives (if available)
   - Verify LOD falls within [0, num_mip_levels]

---

## Code Locations to Review

| File | Lines | Issue | Action |
|------|-------|-------|--------|
| `mipmap.wgsl` | 486 | Bug #1 | Change clamp range |
| `mipmap.wgsl` | 490-520 | Bug #2 | Add atlas scaling |
| `mipmap.wgsl` | 494 | Issue #3 | Fix dimension |
| `mipmap.wgsl` | 5, 520 | Issue #4 | Adjust bias after fixes |
| `mipmap.wgsl` | 507 | Issue #5 | Optional enhancement |

---

## Expected Results After Fixes

| Aspect | Before | After |
|--------|--------|-------|
| **256×256 texture quality** | Good (by accident) | Good (by design) |
| **16×16 texture quality** | Overly blurred | Sharp & correct |
| **2048×2048 texture quality** | Overly blurred | Sharp & correct |
| **Non-square textures** | Incorrect Y axis | Correct both axes |
| **Atlas position variance** | Inconsistent | Consistent |
| **Edge case fallback** | Broken (LOD=0) | Works correctly |
| **LOD value range** | Unpredictable | [0, max_lod] |

---

## Notes

1. **The 0.35 Factor:** Not a magic number but a band-aid
   - `0.35 ≈ 1/2.8 ≈ 1/(sqrt(2)×2)`
   - Appears to be a guess based on empirical observation
   - Happens to work OK for 256×256 textures
   - Breaks for other sizes

2. **Atlas Scaling:** Critical for correctness
   - Each texture occupies a sub-rectangle in the atlas
   - The mipmap ladder is for the entire atlas, not the texture
   - LOD selection must account for this relationship
   - This is **not** just a scaling factor; it's the fundamental math

3. **Why It "Works" Now:**
   - Most textures are 256×256
   - Most are packed at typical atlas positions
   - The 0.35 factor + (-0.5) bias nearly cancel out errors
   - But it's fragile and breaks on different texture sizes

4. **Post-Fix Validation:**
   - Test on diverse texture sets
   - Check that LOD values are mathematically consistent
   - Compare before/after on various zoom levels
   - Look for aliasing reduction or blur changes

