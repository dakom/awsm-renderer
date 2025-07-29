# [live demo](https://dakom.github.io/awsm-renderer)

# WHAT IT HOPES TO BE 

A browser-based Rust/WASM/WebGPU renderer, without using a full game engine framework like Bevy or a modular ECS like Shipyard (bring your own game engine!).

This does *not* use wgpu, but rather uses the WebGPU API directly via the `web-sys` bindings. This is a bit of a departure from the Rust ecosystem, but it allows for a more direct mapping to the WebGPU API and potentially better performance, control, and easier debugging.

# STATUS

Nothing much to see here yet, early days, slow-moving hobby and learning in progress :)

See [ROADMAP](docs/ROADMAP.md) for details.

# DEV

* `just frontend-dev`

# CRATES

* [awsm-renderer](crates/renderer): The renderer in all its glory 
* [awsm-renderer-core](crates/renderer-core): Wraps the WebGPU API with very little opinion, just a nicer Rust API
* [frontend](crates/frontend): Just for demo and debugging purposes 

# MEDIA

For the sake of keeping the repo clean, media files are referenced remotely on the release build, and be downloaded locally to gitignored directories for dev builds. 

See [media/README.md](media/README.md) for more details.

# GRAVEYARD

I've taken some stabs at some variation of this sorta thing before. Some projects got further than others:

* [Pure3d (typescript + webgl1)](https://github.com/dakom/pure3d-typescript)
* [Shipyard ECS (webgl2)](https://github.com/dakom/shipyard-webgl-renderer)
* [WebGL1+2 Rust bindings](https://github.com/dakom/awsm-web/tree/master/crate/src/webgl)

# RENDER PIPELINE

This renderer uses a **visibility buffer+ hybrid** approach, enabling efficient shading, transparency, and TAA without the drawbacks of classic deferred or forward+ rendering.

---

## 🎯 Pipeline Overview

### 1. Geometry Pass (Vertex + Fragment)
- Run for each object with opaque material
- Rendering benefits from hardware occlusion culling, objects are drawn front-to-back
- Outputs to multiple render targets (MRTs):
  - `object_id_texture`: Encodes geometry/material reference per pixel
  - `world_normal_texture`: Interpolated world-space normal
  - `screen_pos_texture`: Clip-space/NDC position for TAA (post-skinning/morphs)
  - `motion_vector_texture`: Computed as difference between current and previous `screen_pos_texture`

> This pass is animated/skinned, so all mesh deformations are already applied before output

### 2. Opaque Shading Pass (Compute Shader)
- Single fullscreen compute dispatch
- For each screen pixel:
  - Read object ID
  - Fetch the material data from a single, large storage buffer
  - Sample the `world_normal_texture` and `screen_pos_texture`
  - Calculate world position by multiplying screen_pos and inverse view-projection
  - Calculate lighting as needed (may use Camera world position etc.)
- Output: shaded color to `opaque_color_texture`

### 3. Transparent Shading Pass (Vertex + Fragment)
- Run for each object with transparent material
- Uses Weighted Blended Order-Independent Transparency (OIT)
- Outputs to multiple render targets (MRTs):
  - `oit_rgb_texture`: accumulated weighted sum of colors
  - `oit_alpha_texture`: accumulated weighted product of transparencies

### 4. Composition (Compute Shader)
- Single fullscreen compute dispatch
- Resolve OIT: Read from `oit_rgb_texture` and `oit_alpha_texture` and calculate the final transparent color.
- Composite: Blend the resolved transparent color over the `opaque_color_texture`.
- Apply TAA: Use `motion_vector_texture` to blend the current, composited frame with the previous frame's history buffer.
- Tonemapping, gamma-correction, etc.
- Outputs final resolved frame 

### 5. Final draw
- Blits the output to screen texture view

---

## 🚦 Render Pass Order

```
1. Geometry Pass         → MRTs (object ID, normals, screen pos, motion vec)
2. Opaque Shading (CS)   → shaded opaque color
3. Transparent Shading   → blended with Weighted Blended OIT
4. Composition (CS)      → final post-processed image
5. Final draw
```

---

## 🆚 Comparison to Deferred Rendering

### Classic Deferred Rendering

Stores full material and geometric data in a G-buffer:

- Albedo, normals, roughness, metalness, emissive, position, etc.
- Lighting applied via fullscreen pixel shader
- Inflexible material model
- Transparency is difficult (deferred blending is hard)

### This Pipeline (Visibility Buffer+)

| Feature              | Deferred Rendering      | Visibility Buffer+        |
| -------------------- | ----------------------- | ------------------------- |
| Shading pass         | Fullscreen pixel shader | Fullscreen compute shader |
| Overdraw for shading | None                    | None                      |
| G-buffer size        | Large (>5 MRTs)         | Medium (4 MRTs)          |
| Material info        | Baked into G-buffer     | Fetched via object ID     |
| Flexibility          | Low                     | High (more dynamic logic) |
| Transparency         | Hard (multiple passes)  | Easy (Weighted OIT pass)  |

> ✅ This approach keeps bandwidth low while still enabling high material complexity and post-processing effects like TAA.

---

## 🔀 Comparison to Forward+

### Forward+ Rendering

- Uses compute pass to build light clusters or screen-space tiles
- Rasterizes geometry and performs per-fragment lighting using local light lists
- Struggles with overdraw and duplicate lighting cost

### Visibility Buffer+

| Feature                     | Forward+           | Visibility Buffer+           |
| --------------------------- | ------------------ | ---------------------------- |
| Light culling               | Clustered per tile | Optional (based on material) |
| Lighting pass               | Per-fragment       | Per-pixel (one-time)         |
| Material evaluation         | During raster      | In compute                   |
| Overdraw impact             | High               | Low                          |
| Transparency                | Native             | Weighted OIT (separate pass) |
| Performance on dense scenes | Degrades           | Stable (no shading overdraw) |

> 🚀 Visibility Buffer+ maintains forward’s material flexibility and transparency support, but eliminates redundant lighting work by separating visibility from shading.

---

## 🧠 Notes

- Compute-based shading benefits from modern GPUs (wave/group operations, coalesced reads)
- Geometry pass performs early-Z and ensures only visible fragments are shaded for opaque objects
- Motion vectors are derived from screen-space movement (ping-ponging previous frame data)
- Transparent objects are shaded in fragment shaders using Weighted Blended OIT to approximate correct order without sorting


# NON-GOALS

### Game world culling

This really depends on the specific needs of a project. Some examples:

* no culling at all (e.g. a fighting game)
* portal-based (e.g. a first-person shooter in an interior) 
* space partitiong (e.g. in an open world game).
* quadtrees (e.g. in top-down view)

However, due to the visibility buffer optimization, the impact of rendering unnecessary geometry does not reach the shading stage.
