# Mipmap Debugging Continued

## What We Did Today

### 1. Switched from LOD to Gradient-Based Sampling
- **Why**: Hardware optimization + automatic anisotropic filtering
- **Implementation**:
  - Added `pbr_get_gradients()` in `mipmap.wgsl` to compute UV derivatives using screen-space Jacobian
  - Changed from `textureSampleLevel()` to `textureSampleGrad()`
  - Simplified derivative calculation from complex world-space reconstruction to elegant 2D screen-space math

### 2. Identified Root Cause: Kaiser Filter Contamination
- Mipmap generation uses Kaiser filter with 4x4 kernel (±2 pixel radius)
- When mipmaps are generated for the entire atlas, the Kaiser filter bleeds adjacent textures into each other
- This contamination is **baked into the mipmap data itself** at generation time

### 3. Attempted Solutions

#### Solution A: Padding-Aware Inset (FAILED)
- Added `padding` field to `TextureInfoRaw` struct (now 40 bytes, was 36)
- Updated `PbrMaterial::BYTE_SIZE` to 268
- Tried to inset the sampling region to avoid contaminated areas
- **Why it failed**: Can't fix mipmaps that are already contaminated

#### Solution B: Accumulated Contamination Formula (FAILED)
- Calculated contamination as: `2 * (2^N - 1)` pixels at mip N
- Tried to inset based on accumulated contamination through mipmap chain
- **Why it failed**: Same issue - contamination is baked in

#### Solution C: Mip Level Clamping (FAILED)
- Formula: `max_safe_mip = log2(padding / 2)`
- With 32px padding: max safe mip = 4
- Clamped mip level and gradients to stay within safe range
- **Why it failed**: Still showing white artifacts

## Current Understanding

### The Core Problem
The Kaiser filter-based mipmap generation and the atlas sampling are **fundamentally incompatible**:

1. **Mipmap generation** (in `mipmap.rs`):
   - Processes entire atlas texture at once
   - Kaiser filter samples 4x4 region (±2 pixels) at each mip level
   - Edge clamping during generation doesn't help - it clamps to atlas boundaries, not individual texture boundaries
   - Contamination happens when generating mip N from mip N-1

2. **Atlas structure** (in `mega_texture.rs`):
   - Multiple textures packed into single 2D array layer
   - Each texture has padding (currently 32px)
   - Padding is written with edge-clamped source pixels
   - But mipmaps are generated for entire atlas, not per-texture

3. **Sampling** (in `textures.wgsl`):
   - Samples from atlas using UV remapping
   - By the time we sample, mipmaps already contain contaminated data
   - No amount of UV clamping or mip clamping can fix pre-contaminated data

### Why Our Solutions Failed
All our solutions tried to **avoid contaminated regions during sampling**, but the contamination is **already in the mipmap pixels**. When you sample mip level 5 at coordinate (x, y), that pixel contains blended data from neighboring atlas textures.

## Next Steps

### Option 1: Simpler Mipmap Generation (RECOMMENDED)
Replace Kaiser filter with standard box/bilinear downsampling:
- Each mip pixel = average of 2x2 region from previous mip
- This is what hardware mipmaps do
- Much simpler, well-understood contamination: only ±1 pixel per mip level
- Contamination formula becomes: `contamination(N) = 2^N - 1` pixels

**Implementation**:
- Modify `mipmap.rs` mipmap shader to use simple 2x2 box filter
- Remove edge detection entirely (it was for thin lines, but we need correctness first)
- This makes padding requirement: `padding >= 2^max_mip - 1`
- With 32px padding: safe up to mip 5 (2^5 - 1 = 31)

### Option 2: Per-Texture Mipmap Generation (COMPLEX)
Generate mipmaps for each texture individually before packing into atlas:
- Generate mipmaps on CPU or in separate GPU pass per texture
- Pack already-mipmapped textures into atlas
- Atlas would store pre-computed mip levels

**Challenges**:
- More complex packing logic
- Potentially more GPU memory (storing individual mip chains)
- Harder to manage dynamic texture loading

### Option 3: Clamp to Mip 0 (TEMPORARY WORKAROUND)
Force all sampling to mip 0 while we fix generation:
```wgsl
let safe_mip_level = 0.0; // Force highest detail
```
This would immediately show if contamination is the issue (should eliminate white artifacts).

### Option 4: Increase Padding Dramatically
With current Kaiser filter:
- Mip 4: needs ~30px padding
- Mip 5: needs ~62px padding
- Mip 6: needs ~126px padding

Try increasing padding to 128px and see if white artifacts move to higher zoom levels.

## Code State

### Files Modified
1. **textures.wgsl** (lines 98-132, 141-180):
   - Added padding field to TextureInfo
   - Implemented mip clamping logic (currently doesn't work)

2. **material.rs** (lines 134-403):
   - Updated BYTE_SIZE to 268
   - Added padding to texture serialization

3. **textures.rs** (line 152):
   - Increased padding from 8 to 32 for testing

4. **compute.wgsl**:
   - Using gradient-based sampling via `pbr_get_material_color_grad()`

5. **mipmap.wgsl**:
   - Refactored to screen-space Jacobian for UV derivatives

### Current Bug Status
- ✅ Gradient calculation working (verified with debug visualization)
- ✅ UVs in valid range (verified with debug visualization)
- ❌ White artifacts persist at all zoom levels
- ❌ Mip clamping doesn't fix the issue

## Recommended Immediate Action

1. **Test if contamination is the cause**:
   ```wgsl
   // In _texture_sample_atlas:
   let safe_mip_level = 0.0;  // Force mip 0

   // In _texture_sample_atlas_grad:
   let clamped_ddx = vec2<f32>(0.0);  // Force mip 0
   let clamped_ddy = vec2<f32>(0.0);
   ```
   If this fixes white artifacts → contamination confirmed
   If this doesn't fix → something else is wrong

2. **If contamination confirmed**, implement simple box filter:
   - Modify `mipmap_shader_source()` in `mipmap.rs`
   - Replace 4x4 Kaiser sampling with 2x2 box filter
   - Remove edge detection pass entirely

3. **Verify the basics**:
   - Check that padding field is being written correctly (add debug logging in Rust)
   - Verify padding value in shader matches what's written from Rust
   - Confirm atlas textures are being written to correct coordinates

## Key Insight

The elegant solution isn't about clamping or insetting during sampling. The elegant solution is to **make mipmap generation respect atlas boundaries from the start**. This means either:
- Using a filter small enough that padding protects us (box filter)
- Generating mipmaps per-texture before atlas packing

The current approach of "generate contaminated mipmaps, then try to avoid them" is fundamentally flawed.
