# IMPLEMENTATION_GUIDELINE.md
## KHR_texture_transform Support (WebGPU / WGSL + Rust + glam)

This document is meant to be **hand-off ready**. It describes how to add KHR_texture_transform-style UV transforms to your renderer, using:

- WebGPU + WGSL on the GPU
- Rust + `glam` on the CPU
- Existing material/texture infrastructure shown in the snippets

Assumptions:

- You already have:
  - `TextureInfoRaw`, `TextureInfo`, `PbrMaterialRaw`, `PbrMaterial` in WGSL
  - A texture pool (2D array textures) + sampler pool
  - `UvDerivs` + `textureSampleGrad` path (compute-based PBR)
  - `PbrMaterial::uniform_buffer_data` in Rust, packing 5 `TextureInfoRaw` blocks
- You want:
  - **Spec-compliant behavior** when pivot/origin = `(0, 0)`
  - Optional pivot (e.g. `(0.5, 0.5)`) with same machinery
  - Efficient, branch-free, trig-free GPU code
  - Global transform table with indices stored inside `TextureInfo`

---

## 1. Math Model & Semantics

KHR_texture_transform defines, per texture slot:

- `offset: [sx, sy]`
- `scale: [sx, sy]`
- `rotation: r` (radians, CCW)
- default rotation origin is **(0, 0)** in UV space

Spec semantics (no custom origin):

```text
uv' = R(r) * (uv * scale) + offset
```

We generalize with an **origin/pivot** `origin = O = (px, py)`:

Conceptually:

```text
uv_local = uv - O
uv_scaled = uv_local * scale
uv_rot    = R * uv_scaled
uv'       = O + uv_rot + offset
```

Let:

- `S = diag(scale.x, scale.y)`
- `R = [[ cos r, -sin r ],
        [ sin r,  cos r ]]`
- `M = R * S` (2×2)
- `O = origin`

You can rewrite:

```text
uv' = O + M * (uv - O) + offset
    = M * uv + (offset + O - M * O)
```

Define:

```text
B = offset + O - M * O
```

This gives the final form we use on the GPU:

```text
uv' = M * uv + B
```

If `origin = (0, 0)` (strict spec behavior):

- `O = 0`, `M * O = 0`, so `B = offset`
- `uv' = M * uv + offset` == KHR_texture_transform spec

So we can always precompute `M` and `B` on the CPU and send only `(M, B)` to the GPU.

---

## 2. GPU-Side Data Structures (WGSL)

### 2.1 Texture transform table

Add a global transform table buffer. Each entry stores:

- `M` (2×2 matrix) as 4 floats `[m00, m01, m10, m11]`
- `B` as 2 floats `[Bx, By]`

```wgsl
struct TextureTransform {
    // M = [ m00  m01 ]
    //     [ m10  m11 ]
    // stored as vec4: (m00, m01, m10, m11)
    m: vec4<f32>;

    // B = offset + origin - M * origin
    b: vec2<f32>;
    _pad: vec2<f32>; // keep 32 bytes total
};

// Adjust group/binding as appropriate
@group(0) @binding(3)
var<storage, read> texture_transforms: array<TextureTransform>;
```

Conventions:

- Entry 0 is **identity**:
  - `M = I = [1,0; 0,1]`
  - `B = (0,0)`
- Any texture usage without KHR_texture_transform uses `transform_index = 0`.

---

### 2.2 `TextureInfoRaw` and `TextureInfo` with `transform_index`

Existing WGSL:

```wgsl
struct TextureInfoRaw {
    // packed: width (low 16 bits), height (high 16 bits)
    size: u32,

    // packed:
    //   bits  0..11 : array_index
    //   bits 12..31 : layer_index
    array_and_layer: u32,

    // packed:
    //   bits  0..7  : uv_set_index
    //   bits  8..31 : sampler_index
    uv_and_sampler: u32,

    // packed:
    //   bits  0..7  : flags
    //   bits  8..15 : address_mode_u
    //   bits 16..23 : address_mode_v
    //   bits 24..31 : padding / reserved
    extra: u32,
};
```

Change the comment to:

```wgsl
// extra:
//   bits  0..7  : flags
//   bits  8..15 : address_mode_u
//   bits 16..23 : address_mode_v
//   bits 24..31 : transform_index
```

And extend `TextureInfo`:

```wgsl
struct TextureInfo {
    size: vec2<u32>,   // (width, height)
    array_index: u32,
    layer_index: u32,
    uv_set_index: u32,
    sampler_index: u32,
    mipmapped: bool,
    address_mode_u: u32,
    address_mode_v: u32,
    transform_index: u32, // NEW
};
```

Update `convert_texture_info`:

```wgsl
fn convert_texture_info(raw: TextureInfoRaw) -> TextureInfo {
    let width:  u32 = raw.size & 0xFFFFu;
    let height: u32 = raw.size >> 16u;

    let array_index: u32 =  raw.array_and_layer & 0xFFFu;
    let layer_index: u32 =  raw.array_and_layer >> 12u;

    let uv_set_index:  u32 =  raw.uv_and_sampler & 0xFFu;
    let sampler_index: u32 =  raw.uv_and_sampler >> 8u;

    let flags: u32          = raw.extra & 0xFFu;
    let mipmapped: bool     = (flags & 0x1u) != 0u;

    let address_mode_u: u32 = (raw.extra >> 8u)  & 0xFFu;
    let address_mode_v: u32 = (raw.extra >> 16u) & 0xFFu;

    let transform_index: u32 = (raw.extra >> 24u) & 0xFFu;

    return TextureInfo(
        vec2<u32>(width, height),
        array_index,
        layer_index,
        uv_set_index,
        sampler_index,
        mipmapped,
        address_mode_u,
        address_mode_v,
        transform_index,
    );
}
```

---

### 2.3 UV transform helper (UV + gradients)

Given your `UvDerivs`:

```wgsl
struct UvDerivs {
    ddx: vec2<f32>;
    ddy: vec2<f32>;
};
```

Add a helper that applies the affine transform to both UV and gradients:

```wgsl
fn apply_texture_transform(
    uv: vec2<f32>,
    derivs: UvDerivs,
    tex_info: TextureInfo
) -> struct {
    uv: vec2<f32>,
    derivs: UvDerivs,
} {
    // Assume index 0 = identity, so no special branch required.
    let t = texture_transforms[tex_info.transform_index];

    let m00 = t.m.x;
    let m01 = t.m.y;
    let m10 = t.m.z;
    let m11 = t.m.w;
    let B   = t.b;

    let uv_transformed = vec2<f32>(
        m00 * uv.x + m01 * uv.y,
        m10 * uv.x + m11 * uv.y
    ) + B;

    let ddx_transformed = vec2<f32>(
        m00 * derivs.ddx.x + m01 * derivs.ddx.y,
        m10 * derivs.ddx.x + m11 * derivs.ddx.y
    );

    let ddy_transformed = vec2<f32>(
        m00 * derivs.ddy.x + m01 * derivs.ddy.y,
        m10 * derivs.ddy.x + m11 * derivs.ddy.y
    );

    let derivs_transformed = UvDerivs(ddx_transformed, ddy_transformed);

    return .{
        uv_transformed,
        derivs_transformed,
    };
}
```

This is **branch-free**, **trig-free**, and works for both identity and non-identity transforms.

---

### 2.4 Where to call `apply_texture_transform` in WGSL

You already compute *attribute* UVs via:

```wgsl
fn texture_uv(
    attribute_data_offset: u32,
    triangle_indices: vec3<u32>,
    barycentric: vec3<f32>,
    tex_info: TextureInfo,
    vertex_attribute_stride: u32
) -> vec2<f32> {
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, triangle_indices.x, vertex_attribute_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, triangle_indices.y, vertex_attribute_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, triangle_indices.z, vertex_attribute_stride);

    return barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;
}
```

Leave this as-is: it gives you **original UVs** in attribute space, which you still need for tangent reconstruction etc.

In each `_pbr_*_color_grad` function, wrap sampling like this:

#### Base color

```wgsl
fn _pbr_material_base_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> vec4<f32> {
    var color = material.base_color_factor;

    if material.has_base_color_texture {
        let result = apply_texture_transform(
            attribute_uv,
            uv_derivs,
            material.base_color_tex_info,
        );

        color *= texture_pool_sample_grad(
            material.base_color_tex_info,
            result.uv,
            result.derivs,
        );
    }

    color.a = 1.0;
    return color;
}
```

#### Metallic-roughness

```wgsl
fn _pbr_material_metallic_roughness_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> vec2<f32> {
    var color = vec2<f32>(material.metallic_factor, material.roughness_factor);

    if material.has_metallic_roughness_texture {
        let result = apply_texture_transform(
            attribute_uv,
            uv_derivs,
            material.metallic_roughness_tex_info,
        );

        let tex = texture_pool_sample_grad(
            material.metallic_roughness_tex_info,
            result.uv,
            result.derivs,
        );

        color *= vec2<f32>(tex.b, tex.g);
    }

    return color;
}
```

#### Normal (only sampling UV is transformed)

```wgsl
fn _pbr_normal_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs,
    world_normal: vec3<f32>,
    barycentric: vec3<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    normal_matrix: mat3x3<f32>,
    os_vertices: ObjectSpaceVertices,
) -> vec3<f32> {
    if !material.has_normal_texture {
        return world_normal;
    }

    let result = apply_texture_transform(
        attribute_uv,
        uv_derivs,
        material.normal_tex_info,
    );

    let tex = texture_pool_sample_grad(
        material.normal_tex_info,
        result.uv,
        result.derivs,
    );

    // tangent_normal etc. unchanged...
    // IMPORTANT: keep tangent reconstruction using attribute UVs (uv0/uv1/uv2)
    // as in your existing code.
    // ...
}
```

#### Occlusion

```wgsl
fn _pbr_occlusion_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> f32 {
    var occlusion = 1.0;

    if material.has_occlusion_texture {
        let result = apply_texture_transform(
            attribute_uv,
            uv_derivs,
            material.occlusion_tex_info,
        );

        let tex = texture_pool_sample_grad(
            material.occlusion_tex_info,
            result.uv,
            result.derivs,
        );

        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }

    return occlusion;
}
```

#### Emissive

```wgsl
fn _pbr_material_emissive_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> vec3<f32> {
    var color = material.emissive_factor;

    if material.has_emissive_texture {
        let result = apply_texture_transform(
            attribute_uv,
            uv_derivs,
            material.emissive_tex_info,
        );

        color *= texture_pool_sample_grad(
            material.emissive_tex_info,
            result.uv,
            result.derivs,
        ).rgb;
    }

    color *= material.emissive_strength;
    return color;
}
```

`pbr_get_material_color_grad` stays mostly unchanged: it still passes `attribute_uv` derived from `texture_uv` into `_pbr_*` functions.

---

## 3. CPU-Side: TextureTransformGpu & Table (Rust + glam)

### 3.1 GPU struct

```rust
use glam::Vec2;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextureTransformGpu {
    // row-major 2x2: [m00, m01, m10, m11]
    pub m: [f32; 4],
    pub b: [f32; 2],
    pub pad: [f32; 2], // keep 32 bytes, matches WGSL (vec4 + vec4)
}

impl TextureTransformGpu {
    pub fn identity() -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 1.0],
            b: [0.0, 0.0],
            pad: [0.0, 0.0],
        }
    }
}

pub fn make_texture_transform(
    offset: Vec2,
    scale: Vec2,
    rotation: f32,
    origin: Vec2, // default Vec2::ZERO for spec behavior
) -> TextureTransformGpu {
    let sx = scale.x;
    let sy = scale.y;
    let ox = offset.x;
    let oy = offset.y;
    let px = origin.x;
    let py = origin.y;

    let c = rotation.cos();
    let s = rotation.sin();

    // M = R * S
    let m00 = c * sx;
    let m01 = -s * sy;
    let m10 = s * sx;
    let m11 =  c * sy;

    // B = offset + origin - M * origin
    let mx_px = m00 * px + m01 * py;
    let my_py = m10 * px + m11 * py;

    let bx = ox + px - mx_px;
    let by = oy + py - my_py;

    TextureTransformGpu {
        m: [m00, m01, m10, m11],
        b: [bx, by],
        pad: [0.0, 0.0],
    }
}
```

### 3.2 Transform table with dedup & u8 indices

```rust
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct TextureTransformKey {
    m: [i32; 4],
    b: [i32; 2],
}

impl TextureTransformKey {
    fn from_gpu(t: &TextureTransformGpu) -> Self {
        const SCALE: f32 = 1_000_000.0;
        let q = |x: f32| (x * SCALE).round() as i32;

        Self {
            m: [q(t.m[0]), q(t.m[1]), q(t.m[2]), q(t.m[3])],
            b: [q(t.b[0]), q(t.b[1])],
        }
    }
}

pub struct TextureTransformTable {
    pub transforms: Vec<TextureTransformGpu>,
    map: HashMap<TextureTransformKey, u8>, // 0..=255
}

impl TextureTransformTable {
    pub fn new() -> Self {
        let mut transforms = Vec::new();
        let mut map = HashMap::new();

        let id = TextureTransformGpu::identity();
        let key = TextureTransformKey::from_gpu(&id);

        transforms.push(id);
        map.insert(key, 0);

        Self { transforms, map }
    }

    pub fn identity_index(&self) -> u8 {
        0
    }

    pub fn get_or_insert(
        &mut self,
        offset: Vec2,
        scale: Vec2,
        rotation: f32,
        origin: Vec2,
    ) -> u8 {
        let gpu = make_texture_transform(offset, scale, rotation, origin);
        let key = TextureTransformKey::from_gpu(&gpu);

        if let Some(&idx) = self.map.get(&key) {
            return idx;
        }

        let idx = self.transforms.len();
        assert!(idx <= u8::MAX as usize, "too many texture transforms (>255)");
        let idx_u8 = idx as u8;

        self.transforms.push(gpu);
        self.map.insert(key, idx_u8);

        idx_u8
    }
}
```

Upload `texture_transforms.transforms` into a storage buffer bound at the WGSL binding.

---

## 4. Integrating with `PbrMaterial` (Rust)

### 4.1 Extend `PbrMaterial` with per-slot transform indices

Add per-slot `u8` transform indices (0 = identity):

```rust
#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub base_color_tex: Option<TextureKey>;
    pub base_color_sampler: Option<SamplerKey>;
    pub base_color_uv_index: Option<u32>;
    pub base_color_factor: [f32; 4];
    pub base_color_transform_index: u8, // NEW

    pub metallic_roughness_tex: Option<TextureKey>;
    pub metallic_roughness_sampler: Option<SamplerKey>;
    pub metallic_roughness_uv_index: Option<u32>;
    pub metallic_factor: f32;
    pub roughness_factor: f32;
    pub metallic_roughness_transform_index: u8, // NEW

    pub normal_tex: Option<TextureKey>;
    pub normal_sampler: Option<SamplerKey>;
    pub normal_uv_index: Option<u32>;
    pub normal_scale: f32;
    pub normal_transform_index: u8, // NEW

    pub occlusion_tex: Option<TextureKey>;
    pub occlusion_sampler: Option<SamplerKey>;
    pub occlusion_uv_index: Option<u32>;
    pub occlusion_strength: f32;
    pub occlusion_transform_index: u8, // NEW

    pub emissive_tex: Option<TextureKey>;
    pub emissive_sampler: Option<SamplerKey>;
    pub emissive_uv_index: Option<u32>;
    pub emissive_factor: [f32; 3];
    pub emissive_strength: f32;
    pub emissive_transform_index: u8, // NEW

    pub vertex_color_info: Option<VertexColorInfo>,
    alpha_mode: MaterialAlphaMode,
    double_sided: bool,
}
```

Initialize to 0 (identity) in `PbrMaterial::new`.

Your glTF loader (or material builder) must:

- Parse KHR_texture_transform for each slot,
- Call `transform_table.get_or_insert(offset, scale, rotation, origin)`,
- Store the returned `u8` in the appropriate `*_transform_index` field.

---

## 5. Updating `pack_texture_info_raw`

Change the function signature to include `transform_index: u8` and pack it in the top 8 bits of `extra`:

```rust
fn pack_texture_info_raw<ID>(
    array: &TexturePoolArray<ID>,
    entry_info: &TexturePoolEntryInfo<ID>,
    uv_index: u32,
    sampler_index: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    transform_index: u8, // NEW
) -> [u32; 4] {
    // size
    let width = array.width;
    let height = array.height;

    debug_assert!(width <= 0xFFFF, "texture width too large for 16 bits");
    debug_assert!(height <= 0xFFFF, "texture height too large for 16 bits");

    let size = (height << 16) | (width & 0xFFFF);

    // array_and_layer
    let array_index = entry_info.array_index as u32;
    let layer_index = entry_info.layer_index as u32;

    debug_assert!(array_index <= 0xFFF, "array_index too large for 12 bits");
    debug_assert!(layer_index <= 0xFFFFF, "layer_index too large for 20 bits");

    let array_and_layer = (layer_index << 12) | (array_index & 0xFFF);

    // uv_and_sampler
    debug_assert!(uv_index <= 0xFF, "uv_index too large for 8 bits");
    debug_assert!(sampler_index <= 0xFFFFFF, "sampler_index too large for 24 bits");

    let uv_and_sampler = (sampler_index << 8) | (uv_index & 0xFF);

    // extra: flags (8) + addr_u (8) + addr_v (8) + transform_index (8)
    let has_mipmaps = array.mipmap;

    let mut flags: u32 = 0;
    if has_mipmaps {
        flags |= 1 << 0;
    }

    debug_assert!(address_mode_u <= 0xFF, "address_mode_u too large for 8 bits");
    debug_assert!(address_mode_v <= 0xFF, "address_mode_v too large for 8 bits");

    let extra =
        (flags & 0xFF)
        | ((address_mode_u & 0xFF) << 8)
        | ((address_mode_v & 0xFF) << 16)
        | ((transform_index as u32) << 24);

    [size, array_and_layer, uv_and_sampler, extra]
}
```

---

## 6. Updating `uniform_buffer_data`

1. Extend the `Value::Texture` variant to carry `transform_index: u8`.
2. Update its `From` impl to accept and store that value.
3. Pass `transform_index` into `pack_texture_info_raw`.
4. For each slot (base color, metallic-roughness, normal, occlusion, emissive), pass the slot’s `*_transform_index` field.

Example for base color block:

```rust
if let Some(tex) = self.base_color_tex.and_then(|texture_key| {
    let entry_info = textures.get_entry(texture_key).ok()?;
    let array = textures.pool.array_by_index(entry_info.array_index)?;
    let sampler_key = self.base_color_sampler?;
    let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
    let uv_index = self.base_color_uv_index?;
    let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);

    Some((
        array,
        entry_info,
        uv_index,
        sampler_index,
        encode_address_mode(address_mode_u),
        encode_address_mode(address_mode_v),
        self.base_color_transform_index, // NEW
    ))
}) {
    write(tex.into());
    bitmask |= Self::BITMASK_BASE_COLOR;
} else {
    write(Value::SkipTexture);
}
```

Repeat for the other texture slots.

`PbrMaterial::BYTE_SIZE` remains **148 bytes** (no change), because `TextureInfoRaw` is still `[u32; 4]` for each of the 5 textures.

---

## 7. Binding the transform buffer

- Create a storage buffer using your existing buffer APIs (e.g. `BufferDescriptor` with `BufferUsage::Storage | BufferUsage::CopyDst`).
- Upload `texture_transforms.transforms` as raw bytes.
- Add a matching binding in your bind group layout + bind group, at the same `@group`/`@binding` as in WGSL.

Example (conceptual; plug into your actual `AwsmRendererWebGpu` abstractions):

```rust
let transform_buffer_desc = BufferDescriptor {
    size: (transform_table.transforms.len() * std::mem::size_of::<TextureTransformGpu>()) as u64,
    usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
    // ...
};

// create buffer, write contents, and bind it where WGSL expects @group(0) @binding(3)
```

---

## 8. Checklist

- [ ] Add `TextureTransform` struct + `texture_transforms` storage buffer in WGSL.
- [ ] Extend `TextureInfo` with `transform_index: u32` and decode from `TextureInfoRaw.extra` bits 24..31.
- [ ] Implement `apply_texture_transform` in WGSL to transform both UV and `UvDerivs` using `texture_transforms[transform_index]`.
- [ ] In `_pbr_*_color_grad` functions, call `apply_texture_transform` before `texture_pool_sample_grad` for each textured slot.
- [ ] Add `TextureTransformGpu` and `make_texture_transform` on the Rust side using `glam::Vec2`.
- [ ] Implement `TextureTransformTable` with a deduplicating `HashMap` and `u8` indices, reserving 0 for identity.
- [ ] Extend `PbrMaterial` with per-slot `u8` transform indices, defaulting to 0 (identity).
- [ ] Update `pack_texture_info_raw` to take `transform_index: u8` and pack it into the top 8 bits of `extra`.
- [ ] Update `PbrMaterial::uniform_buffer_data` to pass the correct per-slot `*_transform_index` down to `pack_texture_info_raw`.
- [ ] Allocate and upload the transform table to the GPU, and bind it where WGSL expects it.

When these steps are complete, you’ll have:

- Proper KHR_texture_transform semantics (scale → rotate → offset about origin (0,0) by default).
- Optional arbitrary origin support with no extra shader cost.
- Per-slot transforms (per `TextureInfo`), matching glTF semantics.
- Efficient, branch-free, trig-free GPU sampling code.
