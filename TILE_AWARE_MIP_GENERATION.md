# Tile-Aware Mipmap Generation for Texture Atlases (with Safe Gutters)

This document describes how to generate mipmaps for texture atlases **without bleeding between tiles**, even under **high anisotropy (up to 16×)**. It includes a WGSL compute kernel and detailed guidance for gutter sizing and mip generation.

---

## 🎯 Goal

Create mipmaps for a **texture atlas** where:
- Each tile has its own padded border (gutter).
- Mipmaps are generated **per tile**, not globally.
- Each level duplicates edges into its gutter.
- Sampling with anisotropic filtering remains artifact-free.

---

## 🧠 Background

Standard mip generation (e.g. using a Kaiser filter) samples texels outside tile boundaries, pulling in neighbor colors. When these are packed into an atlas, this causes **haloing and seams**.

The fix: a **tile-aware mip generator** that:
1. Reads texels clamped to the tile’s interior.
2. Writes to a destination rect that includes a **gutter**.
3. Repeats for each mip level, maintaining edge-extended padding.

---

## 📏 Choosing the Gutter Size

| Filtering | Max filter width | Safe gutter per level | Notes |
|------------|------------------|-----------------------|--------|
| Trilinear only | ~1 texel | 1 | Minimum for smooth transitions. |
| 4× AF | ~4 texels | 2 | Handles moderate anisotropy. |
| 8× AF | ~8 texels | 3 | Safe for high-quality renderers. |
| **16× AF** | **~8–10 texels** | **4–8 texels** | Recommended for extreme oblique angles. |

### ✅ Recommendation for large atlases
Use **8 texel gutters** per level. This guarantees safety for 16× AF and only adds a few percent texture overhead.

#### Memory overhead examples

| Tile size | Gutter (px) | Approx overhead |
|------------|--------------|------------------|
| 512×512 | 8 | +6% |
| 1024×1024 | 8 | +3% |
| 256×256 | 8 | +13% |

---

## ⚙️ WGSL: Tile-Aware Mip Generator (Variable Gutter)

This kernel downsamples one mip level into the next while clamping reads to a tile’s interior and writing gutters around the output tile.

```wgsl
// --- Input: parent mip level (L), Output: child mip level (L+1) ---
// Use rgba8unorm here; adjust for your format.
@group(0) @binding(0) var src : texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var dst : texture_storage_2d<rgba8unorm, write>;

struct Rect { min: vec2<i32>; max: vec2<i32>; }; // [min, max)

struct Params {
  srcInteriorMin : vec2<i32>; // parent level interior (no gutter)
  srcInteriorMax : vec2<i32>;
  gutter         : i32;       // per-level gutter (e.g., 8)
};
@group(0) @binding(2) var<uniform> U : Params;

fn clamp_to_rect(p: vec2<i32>, r: Rect) -> vec2<i32> {
  return clamp(p, r.min, r.max - vec2<i32>(1));
}

@compute @workgroup_size(8,8)
fn mip_tile_downsample(@builtin(global_invocation_id) gid: vec3<u32>) {
  // Compute child interior and padded rect
  let childInteriorMin = U.srcInteriorMin / 2;
  let childInteriorMax = (U.srcInteriorMax + vec2<i32>(1,1)) / 2;
  let g = vec2<i32>(U.gutter, U.gutter);
  let dstRectMin = childInteriorMin - g;
  let dstRectMax = childInteriorMax + g;

  let d = vec2<i32>(gid.xy) + dstRectMin;
  if (any(d < dstRectMin) || any(d >= dstRectMax)) { return; }

  // Clamp destination to interior for gutter filling
  let d_clamped = clamp(d, childInteriorMin, childInteriorMax - vec2<i32>(1));

  // Map to parent texels
  let s_base = 2 * d_clamped;
  let srcRect : Rect = Rect(U.srcInteriorMin, U.srcInteriorMax);

  let s00 = clamp_to_rect(s_base + vec2<i32>(0,0), srcRect);
  let s10 = clamp_to_rect(s_base + vec2<i32>(1,0), srcRect);
  let s01 = clamp_to_rect(s_base + vec2<i32>(0,1), srcRect);
  let s11 = clamp_to_rect(s_base + vec2<i32>(1,1), srcRect);

  let c = (textureLoad(src, s00).rgba +
           textureLoad(src, s10).rgba +
           textureLoad(src, s01).rgba +
           textureLoad(src, s11).rgba) * 0.25;

  textureStore(dst, d, c);
}
```

---

## 🧮 Dispatch Logic

For each atlas tile and mip level `L → L+1`:

1. `srcInterior = [cellOrigin_L, cellOrigin_L + cellSize_L)`  
2. Set `gutter = 8`
3. Dispatch this compute kernel for the destination rect:  
   `[childInteriorMin - g, childInteriorMax + g)`  
   where `childInterior = srcInterior / 2` (rounded up).
4. Repeat per level.

Each level’s output becomes the next level’s input; the gutters naturally propagate down.

---

## 🧰 Variants by Texture Type

These are meant as examples, we can explore it more consciously later:

| Texture Type | Special handling |
|---------------|------------------|
| **sRGB Albedo** | Use `textureSampleGrad()`. |
| **Normal maps** | Average linearly, **renormalize** vector per texel. |
| **Roughness / Gloss** | Average `r²`, then take `sqrt`. |
| **Alpha-tested / cutouts** | Preserve coverage; bias alpha or compute coverage mip. |

---

## ⚙️ Sampling setup

At runtime, always use a **ClampToEdge sampler** for the atlas.  
If you need wrap/repeat, apply it in the *pre-atlas UVs* before remapping to atlas space.

For mipmap sampling (in fragment or compute):
- Use `textureSampleGrad()` with explicit gradients

---

## ✅ Results

- Seam-free mips under trilinear and 16× AF.
- Supports color, normal, ORM, and alpha textures.
- Minimal overhead (<10% for large tiles).

---

## 💡 Key Takeaways

- 1–2 texels per level is fine for most use cases.  
- For 16× AF, **4–8 texels** guarantees no bleed.  
- Per-tile mip generation with clamped reads is the only way to stay artifact-free.  
- Kaiser filters are great for resizing a single image — **not** for atlas mip-chains.
