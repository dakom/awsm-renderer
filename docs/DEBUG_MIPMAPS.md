# Mipmap Gradient Investigation - Next Steps

## Current Status

We've systematically verified almost every component of the gradient calculation pipeline, but still see textures appearing too blurry (atlas mip 2-3 instead of expected mip 0-1).

### What We've Verified ✓

1. **Screen-space coordinate system**: Hardware `dFdx(screen_pos)` matches expected (1,0) and (0,1) → **Correct**
2. **Barycentric gradients**: Hardware `dFdx(barycentric)` matches our geometric calculation → **Correct** (ratio ~1.0)
3. **Screen-space positions**: Hardware screen positions match our `clip_to_pixel` transformation → **Correct** (< 0.5 pixel error)
4. **Texel:pixel geometry**: Area calculations show 4:1 texel:pixel area ratio (2:1 linear) → **Correct** for expected mip 1

### The Mystery

Despite all individual components being verified correct, we're still selecting atlas mip 2-3 when geometry suggests we should select mip 1.

**Hypothesis**: The issue is in the UV gradient calculation specifically. We've verified barycentric gradients match hardware, but we haven't directly compared **UV gradients** (hardware `dFdx(uv)` vs our geometric calculation).

---

## What We Need: Forward Rendering Pass with UV Data

To complete the investigation, you need a rendering pass that:

1. **Has UV coordinates available in the vertex shader** (either as vertex attribute or readable from storage buffer)
2. **Computes hardware UV gradients in fragment shader** using `dFdx(uv)` and `dFdy(uv)`
3. **Outputs those gradients to a texture** (similar to how we tested barycentric gradients)
4. **Allows comparison with our geometric calculation** in the compute shader

The **alpha materials forward pass** is perfect for this because:
- It will already have proper UV binding for texture sampling
- It's a fragment shader pass (has automatic derivatives)
- We can temporarily hijack one of its outputs for debugging

---

## Setup Instructions (Once Alpha Pass is Ready)

### 1. Fragment Shader Changes

In your alpha pass fragment shader, add:

```wgsl
@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    // ... existing code ...

    // DEBUG: Compute hardware UV gradients
    let uv = input.uv;  // Or however you get UV in your fragment shader
    let ddx_uv = dpdx(uv);
    let ddy_uv = dpdy(uv);

    // Output to a debug texture (pick an unused output or temporarily hijack one)
    out.debug_output = vec4<f32>(ddx_uv, ddy_uv);

    // ... rest of code ...
}
```

### 2. Compute Shader Comparison

In your material compute shader (or a debug compute pass), add:

```wgsl
// Read hardware UV gradients from alpha pass output
let hw_uv_grads = textureLoad(debug_texture, coords, 0);
let hw_ddx_uv = hw_uv_grads.xy;
let hw_ddy_uv = hw_uv_grads.zw;

// Compute geometric UV gradients using our method
let uv0 = /* get UV for vertex 0 */;
let uv1 = /* get UV for vertex 1 */;
let uv2 = /* get UV for vertex 2 */;

// Transform triangle to screen space
let mvp = camera.view_proj * world_model;
let clip0 = mvp * vec4<f32>(os_vertices.p0, 1.0);
let clip1 = mvp * vec4<f32>(os_vertices.p1, 1.0);
let clip2 = mvp * vec4<f32>(os_vertices.p2, 1.0);

let p0 = clip_to_pixel(clip0, screen_size);
let p1 = clip_to_pixel(clip1, screen_size);
let p2 = clip_to_pixel(clip2, screen_size);

// Compute barycentric derivatives (already verified correct!)
let bary_grads = compute_barycentric_derivatives(p0, p1, p2);

// Apply chain rule: d(UV)/d(screen) = d(UV)/d(bary) × d(bary)/d(screen)
let duv_db1 = uv1 - uv0;
let duv_db2 = uv2 - uv0;
let geo_ddx_uv = duv_db1 * bary_grads.x + duv_db2 * bary_grads.z;
let geo_ddy_uv = duv_db1 * bary_grads.y + duv_db2 * bary_grads.w;

// Compare magnitudes
let hw_mag = max(length(hw_ddx_uv), length(hw_ddy_uv));
let geo_mag = max(length(geo_ddx_uv), length(geo_ddy_uv));
let ratio = geo_mag / max(hw_mag, 1e-8);

// Visualize ratio
if (ratio > 0.9 && ratio < 1.1) {
    color = vec3<f32>(0.0, 1.0, 0.0);  // Green = perfect match!
} else if (ratio > 1.1) {
    color = vec3<f32>(1.0, 0.0, 0.0);  // Red = geometric too large
} else {
    color = vec3<f32>(0.0, 0.0, 1.0);  // Blue = geometric too small
}
```

---

## Expected Results and Next Steps

### Case 1: Green (ratio ~1.0) - Gradients Match

**Meaning**: Our geometric UV gradient calculation is mathematically correct and matches hardware!

**Implications**:
- The 4x discrepancy is NOT in our gradient calculation
- The issue is elsewhere in the pipeline (atlas transform? `textureSampleGrad` interpretation? sampler settings?)

**Next steps**:
1. Verify atlas UV transform is applied correctly (multiply gradients by `uv_scale`?)
2. Check if `textureSampleGrad` has different expectations for gradient units
3. Compare against reference glTF viewer's approach
4. May need to accept empirical scale factor (0.25) as architectural difference

### Case 2: Red (ratio > 1.1) - Geometric Too Large

**Meaning**: Our geometric calculation produces gradients that are systematically larger than hardware.

**Common causes**:
- Missing divide by 2 (hardware may compute over 2x2 quads differently)
- Screen-space units mismatch (half-pixels? different coordinate system?)
- Barycentric interpolation difference (though we verified this separately)

**Next steps**:
1. Calculate exact ratio (e.g., 2x, 4x)
2. Add compensating scale factor: `geo_ddx_uv / ratio`
3. Verify it works across different models and viewing distances
4. Document why hardware differs from geometric calculation

### Case 3: Blue (ratio < 0.9) - Geometric Too Small

**Meaning**: Our geometric calculation produces gradients that are systematically smaller than hardware.

**Common causes**:
- Missing multiply by 2 (hardware uses quad deltas, we use per-pixel)
- Atlas coordinate system issue (tile-space vs atlas-space)
- Perspective correction not applied when needed

**Next steps**:
1. Calculate exact ratio
2. Add compensating scale factor: `geo_ddx_uv * (1.0 / ratio)`
3. Test across models
4. Document the correction

---

## Test Procedure

1. **Pick a simple test case**: A quad with a single texture mapped [0,1] in UV space
2. **Position camera** so quad fills ~100x100 pixels on screen
3. **Run comparison** and note the color
4. **Zoom in/out** and verify ratio stays consistent (should be constant per-triangle)
5. **Try different models** to ensure it's not model-specific

---

## Files Modified During Investigation

### Shader Files (will need cleanup after debug)

- `/crates/renderer/src/render_passes/geometry/shader/geometry_wgsl/fragment.wgsl`
  - Currently outputs debug data in `geometry_tangent`
  - Restore original: `out.geometry_tangent = vec4<f32>(normalize(input.world_tangent.xyz), input.world_tangent.w);`

- `/crates/renderer/src/render_passes/geometry/shader/geometry_wgsl/vertex.wgsl`
  - Currently has debug UV reading code
  - Remove `get_vertex_uv()` function and restore original tangent output

- `/crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/compute.wgsl`
  - Has various debug visualizations (lines 420-482)
  - Current active: position verification debug
  - Clean up and restore normal rendering

- `/crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`
  - Updated to use chain-rule approach (correct!)
  - Removed `MIPMAP_GRADIENT_SCALE` constant
  - Keep these changes - they're improvements!

### Reference Documents Created

- `/tmp/MIPMAP_INVESTIGATION.md` - Summary of findings
- `/tmp/mipmap_debug.wgsl` - Texel:pixel ratio debug code
- `MIPMAP_TAKE_3.md` - Your original orthographic vs perspective guide

---

## Quick Reference: Mip Level Color Codes

When visualizing atlas mip levels:
- **Blue** = Atlas mip 0 (full resolution)
- **Green** = Atlas mip 1 (half resolution)
- **Yellow-green** = Atlas mip 2 (quarter resolution)
- **Yellow** = Atlas mip 3
- **Orange** = Atlas mip 4
- **Red** = Atlas mip 5+

For a 1024x1024 tile in 4096x4096 atlas viewed at 1:1 texel:pixel ratio, expect **blue** (mip 0).

---

## The Bottom Line

We've verified everything except the final piece: **do our computed UV gradients match hardware UV gradients?**

Once you have the alpha pass with proper UV data, run the comparison test. The color you see (green/red/blue) will definitively tell us:
- **Green**: Our math is perfect, issue is elsewhere (accept it or investigate atlas/sampler)
- **Red/Blue**: Our math needs a scale correction (calculate ratio, apply fix, document why)

Either way, you'll have a concrete answer and path forward!

---

## Alternative: Accept Current State

Given that we've verified so many components as correct, you could also:

1. **Accept the empirical 0.25 scale factor** for the specific 1024/4096 atlas ratio
2. **Make it data-driven**: `scale = tile_size / atlas_size`
3. **Test if it generalizes** to different tile/atlas size combinations
4. **Document it as "empirical calibration required for compute shader gradient approximation"**

This is pragmatic, but doesn't explain the root cause. The alpha pass test will definitively solve the mystery.
