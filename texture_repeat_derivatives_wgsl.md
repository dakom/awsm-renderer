# Correct Handling of Derivatives for Repeated Textures in WebGPU (WGSL)

## Overview

When working with **explicit gradients** (`textureSampleGrad`) and **repeated textures**, you must ensure that **texture coordinates and gradients are handled separately**.  
Wrapping (tiling/repetition) should apply only to **coordinates**, not to the **gradients**.

Incorrect handling—such as taking `fract(uv)` or `mod(uv)` *before* sampling—causes **derivative discontinuities** and **LOD errors** (e.g., shimmering or popping).

---

## ⚠️ Common Pitfall

```wgsl
// ❌ Wrong: wrapping before computing LOD
let uv_wrapped = fract(uv);
let color = textureSampleGrad(tex, sampRepeat, uv_wrapped, dpdx(uv_wrapped), dpdy(uv_wrapped));
```

- The `fract()` introduces jumps between 0 and 1 at each repeat.  
- The derivatives at those discontinuities explode or become undefined.  
- The mip selection becomes unstable near seams.

---

## ✅ Correct Solution

Keep a **continuous (unwrapped)** UV for derivatives, and **only wrap the coordinates** passed to the texture lookup.

### Example with Repeat Sampler

```wgsl
// Continuous UVs and their derivatives (from previous passes or computed)
let uv_cont   : vec2<f32> = uv;
let ddx_cont  : vec2<f32> = derivs.ddx;
let ddy_cont  : vec2<f32> = derivs.ddy;

// Apply tiling to both coords and grads
let tiling : vec2<f32> = vec2(tiles_u, tiles_v);
let uv_t   = uv_cont * tiling;
let ddx_t  = ddx_cont * tiling;
let ddy_t  = ddy_cont * tiling;

// Sample with a sampler that has `addressMode: "repeat"`
let color = textureSampleGrad(tex, sampRepeat, uv_t, ddx_t, ddy_t);
```

**Notes:**
- The sampler handles the wrap.  
- Gradients remain smooth because they’re from the continuous coordinate space.  

---

### Example with Manual Wrapping (Atlas Case)

If you must wrap manually (e.g., you use a texture atlas and a clamped sampler), split coordinate wrapping and gradient calculation:

```wgsl
// Continuous coords and grads first
let uv_cont  = uv;
let ddx_cont = derivs.ddx;
let ddy_cont = derivs.ddy;

// Apply tiling to both coords and grads
let tiling = vec2<f32>(tiles_u, tiles_v);
let uv_scaled   = uv_cont * tiling;
let ddx_scaled  = ddx_cont * tiling;
let ddy_scaled  = ddy_cont * tiling;

// Wrap only the coordinates
let uv_wrapped = fract(uv_scaled);

// Map into atlas sub-rectangle
let sub_size   = vec2<f32>(rect_w, rect_h);
let sub_offset = vec2<f32>(rect_x, rect_y);
let uv_atlas   = uv_wrapped * sub_size + sub_offset;
let ddx_atlas  = ddx_scaled * sub_size;
let ddy_atlas  = ddy_scaled * sub_size;

// Sample
let color = textureSampleGrad(tex, sampClamp, uv_atlas, ddx_atlas, ddy_atlas);
```

**Key principle:**  
> Only wrap the coordinates. Never wrap or modify the gradients.

---

## 🧠 Optional: Quad-Consistent Wrapping

When emulating repeat manually, avoid tiny seams between quads by rebasing to a consistent tile index:

```wgsl
// Estimate the quad center UV to pick a common tile index
let uv_center = uv_cont - 0.5 * (ddx_cont + ddy_cont);
let base_tile = floor(uv_center);

// Rebase UVs to [0, 1) in a continuous manner
let uv_rebased = uv_cont - base_tile;

// Gradients remain unchanged
let ddx_rebased = ddx_cont;
let ddy_rebased = ddy_cont;

// Sample
let color = textureSampleGrad(tex, sampClamp, uv_rebased, ddx_rebased, ddy_rebased);
```

This prevents adjacent pixels in the same quad from landing in different wrapped tiles, which causes seams or mip glitches.

---

## ✅ Quick Checklist

| ✅ Do | ❌ Don’t |
|-------|----------|
| Apply tiling to both `uv` and gradients. | Apply `fract` or `mod` to the same UVs you use for LOD. |
| Wrap only the coordinate before sampling. | Wrap gradients. |
| Prefer samplers with `addressMode: "repeat"`. | Rely on `fract(uv)` for repetition. |
| Keep gradients in continuous space. | Compute gradients from wrapped coordinates. |
| Convert both coords & grads to atlas space together. | Scale only the coords, not the grads. |

---

## 🧩 Summary

- **Problem:** Repeated textures cause incorrect mipmap LODs when you wrap (`fract`) coordinates before computing derivatives.  
- **Fix:** Derive gradients from **continuous UVs**; only wrap the coordinates before sampling.  
- **Result:** Smooth mip transitions, correct LOD, no shimmering or popping.

