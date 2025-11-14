# Texture Pool sRGB Conversion Optimization

## Context - What Was Fixed Today

### Bug #1: Wrong Texture Format ✅ Fixed
**File:** `crates/renderer/src/gltf/populate/material.rs:240`
- Was hardcoding `TextureFormat::Rgba16float` instead of using `image_data.format()`
- Fixed to use the correct format (`Rgba8unorm` for regular images)

### Bug #2: Struct Size Mismatch ✅ Fixed
**Files:**
- `crates/renderer/src/materials/pbr/material.rs:67-69`
- `crates/renderer/src/render_passes/material/shared/shader/pbr_shared_wgsl/material.wgsl:23,36-37`

Changed `TextureInfoRaw` from 64 bytes (old atlas format) to 16 bytes (new packed format):
- Updated `BYTE_SIZE` from 388 to 148 bytes
- Updated shader padding from `array<u32, 31>` to `array<u32, 91>`
- Fixed memory alignment between CPU and GPU

### Bug #3: Missing sRGB Conversion ⚠️ Temporary Fix (Needs Optimization)
**File:** `crates/renderer/src/render_passes/material/shared/shader/all_material_shared_wgsl/textures.wgsl`

Added shader-based sRGB→linear conversion in:
- `_texture_pool_sample_grad()` (lines 153-156)
- `_texture_pool_sample_no_mips()` (lines 202-205)

**This works but has performance costs:**
- Branch check per texture sample (causes divergence)
- `pow(x, 2.4)` operations × 3 RGB channels per texture sample
- For typical PBR material (5 textures), this is ~2-3 pow operations per pixel, per frame
- At 60fps @ 1080p, this is expensive

---

## What Needs To Be Done: Upload-Time sRGB Conversion

### Goal
Move sRGB→linear conversion from per-frame sampling to one-time upload (like the old mega_texture system did).

### Reference Implementation
The old `mega_texture` system did this correctly:
- **File:** `crates/renderer-core/src/texture/mega_texture/shader.wgsl:44-46`
- Converted during upload compute pass, before storing to atlas
- Zero runtime cost during rendering

### Implementation Location
**File:** `crates/renderer-core/src/texture/texture_pool.rs:170-233`
**Function:** `TexturePoolArray::write_gpu()`

### Current Flow
```rust
1. Create GPU texture (lines 184-193)
2. Copy images to mip level 0 (lines 197-212)
   - Uses `copy_external_image_to_texture()` for each layer
3. Generate mipmaps if needed (lines 214-216)
4. Create texture view (lines 218-226)
```

### New Flow (Add Step 2.5)
```rust
1. Create GPU texture
2. Copy images to mip level 0 (as-is, still sRGB for relevant textures)
2.5. **NEW: Run sRGB→linear conversion compute pass**
    - Only for layers where `color.srgb_encoded == true`
    - Reads from mip level 0, converts, writes back to mip level 0
3. Generate mipmaps (now from correct linear data)
4. Create texture view
```

---

## Implementation Steps

### Step 1: Create Compute Shader for In-Place Conversion
Create a new shader that reads/writes to the same texture layer at mip level 0.

**Location:** Probably add to `crates/renderer-core/src/texture/texture_pool/` as `convert_srgb.rs` or add to existing `mipmap.rs`

**Shader needs:**
```wgsl
@group(0) @binding(0) var src: texture_2d_array<f32>;
@group(0) @binding(1) var dst: texture_storage_2d_array<rgba8unorm, write>;

struct Params {
    layer: i32,
    width: u32,
    height: u32
}

@group(0) @binding(2) var<uniform> params: Params;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(vec3<f32>(0.04045), c);
    let low  = c / 12.92;
    let high = pow(max((c + 0.055) / 1.055, vec3<f32>(0.0)), vec3<f32>(2.4));
    return mix(low, high, cutoff);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }

    let coord = vec2<i32>(gid.xy);
    var color = textureLoad(src, coord, params.layer, 0);

    // Convert RGB, preserve alpha
    color = vec4<f32>(srgb_to_linear(color.rgb), color.a);

    textureStore(dst, coord, params.layer, color);
}
```

**IMPORTANT NOTE:** You can reuse the existing `srgb_to_linear()` function from `color_space.wgsl` - just include it.

### Step 2: Add Conversion Pass to `write_gpu()`

**Location:** `crates/renderer-core/src/texture/texture_pool.rs:214` (right before mipmap generation)

```rust
// After copying all images (line 212), before mipmaps (line 214):

// Convert sRGB to linear for textures that need it
for (index, (_, _, color)) in self.images.iter().enumerate() {
    if color.srgb_encoded {
        convert_srgb_to_linear(
            gpu,
            &dest_tex,
            index as u32,
            self.width,
            self.height
        ).await?;
    }
}

// Then generate mipmaps as normal (existing line 214-216)
if self.mipmap {
    generate_mipmaps(gpu, &dest_tex, &mipmap_texture_kinds, mipmap_levels).await?;
}
```

### Step 3: Implement `convert_srgb_to_linear()` Helper Function

**Pattern:** Follow the same structure as `generate_mipmaps()` in `mipmap.rs`:
- Create/cache pipeline with bind group layout
- Create bind groups for src/dst texture views and params uniform
- Run compute pass

**Key details:**
- Source texture: same as destination, mip level 0
- Destination: storage texture view, same layer, mip level 0
- Need to create TWO views of the same texture:
  - One for reading (sampled texture)
  - One for writing (storage texture)

### Step 4: Remove Shader-Based Conversion

**File:** `crates/renderer/src/render_passes/material/shared/shader/all_material_shared_wgsl/textures.wgsl`

**Remove these blocks from both functions:**
```wgsl
// Convert sRGB to linear if needed
if info.srgb {
    color = vec4<f32>(srgb_to_linear(color.rgb), color.a);
}
```

From:
- `_texture_pool_sample_grad()` (currently lines 153-156)
- `_texture_pool_sample_no_mips()` (currently lines 202-205)

### Step 5: Remove `srgb` Field from `TextureInfo` (Optional Cleanup)

**Files:**
- `crates/renderer/src/render_passes/material/shared/shader/all_material_shared_wgsl/textures.wgsl:17`
- `crates/renderer/src/render_passes/material/shared/shader/all_material_shared_wgsl/textures.wgsl:42,49`

Since textures are now always linear in GPU memory, the `srgb` flag in `TextureInfo` is no longer needed for sampling. However, you might want to keep it for debugging/validation purposes.

**Decision:** Probably keep the flag but don't use it in sampling code. The struct size is already fixed, and the flag could be useful for debugging.

---

## Important Notes

### Which Textures Need Conversion?

**DO convert (sRGB in files → linear in GPU):**
- Base color / albedo textures
- Emissive textures

**DO NOT convert (already linear in files):**
- Normal maps
- Metallic/roughness maps
- Occlusion maps
- Height maps

The `color.srgb_encoded` flag is already being set correctly in:
- `crates/renderer/src/gltf/populate/material.rs:176-233`

### TextureInfo.srgb Flag

After upload-time conversion is implemented:
- ✅ All textures in GPU memory are in linear space
- ✅ The `srgb` flag in `TextureInfo` becomes redundant for sampling
- ✅ Can remove the shader branching code entirely
- ⚠️ Consider keeping the flag for debugging/validation but ignoring it in sampling

### Texture Usage Flags

Make sure the texture has both:
- `TEXTURE_BINDING` (for reading)
- `STORAGE_BINDING` (for writing in compute shader)

Already set correctly in `TEXTURE_USAGE_MIPMAP` constant (lines 237-244).

### Performance Benefits

**Before (current):**
- Cost: ~2-3 `pow(x, 2.4)` operations per pixel, per frame
- Branching divergence when mixing sRGB/linear textures

**After:**
- Cost: One-time compute pass per texture upload
- Zero per-frame cost
- No branching in hot rendering path

For a 1024×1024 texture:
- Upload cost: 1,048,576 pixels × 1 time = ~1M operations once
- Frame cost saved: resolution × ~2-3 pow operations × 60fps

---

## Testing Checklist

After implementation:
- [ ] Colors look correct (not washed out)
- [ ] sRGB textures (base color, emissive) are properly converted
- [ ] Linear textures (normal, roughness) are NOT converted
- [ ] Mipmaps are generated from correct linear data
- [ ] No performance regression in rendering loop
- [ ] Build succeeds
- [ ] Shader no longer has sRGB conversion branches

---

## Additional Context

### Why Rgba8unorm Instead of Rgba8unormSrgb?

We use `Rgba8unorm` (not `Rgba8unormSrgb`) because:
1. sRGB formats don't support `STORAGE_BINDING` usage
2. Mipmap generation needs write access via storage texture
3. We handle conversion explicitly for full control

This is documented in `crates/renderer-core/src/image.rs:67-73`.

### References

- Old mega_texture implementation: `crates/renderer-core/src/texture/mega_texture/shader.wgsl:44-46`
- Mipmap generation pattern: `crates/renderer-core/src/texture/mipmap.rs`
- Current upload flow: `crates/renderer-core/src/texture/texture_pool.rs:170-233`

---

## Tomorrow's Prompt

"Please read TO_BE_CONTINUED.md and implement the upload-time sRGB→linear conversion for the texture pool system as described."
