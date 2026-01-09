# Materials: Adding New Shader Features

This document explains how to add new material features (like glTF extensions) to the renderer. The system spans both Rust (data loading, GPU upload) and WGSL (shader calculations).

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Rust Side                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  gltf/populate/material.rs    →  Parse glTF, populate PbrMaterial fields    │
│  materials/pbr/material.rs    →  PbrMaterial struct, uniform_buffer_data()  │
│  materials/pbr/buffers.rs     →  GPU buffer management                      │
│  render_passes/.../template.rs →  Shader cache keys for immutable variants  │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓ GPU Upload
┌─────────────────────────────────────────────────────────────────────────────┐
│                              WGSL Side                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  shared/.../material.wgsl        →  PbrMaterial struct (mirrors Rust)       │
│  shared/.../material_color.wgsl  →  PbrMaterialColor (sampled values)       │
│  opaque/.../material_color_calc  →  Sampling (grad + no_mips variants)      │
│  transparent/.../material_color_calc →  Sampling (fragment shader)          │
│  shared/.../brdf.wgsl            →  Lighting calculations                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Step-by-Step: Adding a New Material Feature

### 1. Parse from glTF (`gltf/populate/material.rs`)

Extract the extension data from the glTF document and populate the material:

```rust
// In pbr_material_mapper()
if let Some(my_extension) = gltf_material.my_extension() {
    material.my_feature_factor = my_extension.factor();
    material.my_feature_color = my_extension.color();

    // Handle textures
    if let Some(tex) = my_extension.texture().map(GltfTextureInfo::from) {
        let GLtfMaterialCacheKey { uv_index, texture_key, sampler_key, texture_transform_key } =
            tex.create_material_cache_key(
                renderer, ctx,
                TextureColorInfo {
                    mipmap_kind: MipmapTextureKind::MyFeature,
                    srgb_to_linear: true,  // depends on texture semantics
                    premultiplied_alpha,
                },
            ).await?;

        material.my_feature_tex = Some(texture_key);
        material.my_feature_sampler = Some(sampler_key);
        material.my_feature_uv_index = Some(uv_index as u32);
        material.my_feature_texture_transform = texture_transform_key;
    }
}
```

### 2. Add Fields to PbrMaterial (`materials/pbr/material.rs`)

```rust
#[derive(Clone, Debug)]
pub struct PbrMaterial {
    // ... existing fields ...

    // my_extension
    pub my_feature_tex: Option<TextureKey>,
    pub my_feature_sampler: Option<SamplerKey>,
    pub my_feature_uv_index: Option<u32>,
    pub my_feature_texture_transform: Option<TextureTransformKey>,
    pub my_feature_factor: f32,
    pub my_feature_color: [f32; 3],

    immutable: PbrMaterialImmutable,
}
```

Add defaults in `PbrMaterial::new()`:

```rust
my_feature_tex: None,
my_feature_sampler: None,
my_feature_uv_index: None,
my_feature_texture_transform: None,
my_feature_factor: 1.0,  // sensible default
my_feature_color: [1.0, 1.0, 1.0],
```

### 3. Add Bitmask Constant

```rust
impl PbrMaterial {
    // ... existing bitmasks ...
    pub const BITMASK_MY_FEATURE: u32 = 1 << 8;  // next available bit
```

### 4. Update uniform_buffer_data()

This serializes the material to bytes for GPU upload. **Order matters** - must match the WGSL struct exactly.

```rust
fn uniform_buffer_data(&self, textures: &Textures) -> Result<[u8; Self::BYTE_SIZE]> {
    // ... existing writes ...

    // Write scalar factors (f32 values)
    write(self.my_feature_factor.into());
    write(self.my_feature_color[0].into());
    write(self.my_feature_color[1].into());
    write(self.my_feature_color[2].into());

    // Write texture (20 bytes packed format)
    if let Some(tex) = self.my_feature_tex.and_then(|texture_key| {
        // ... same pattern as other textures ...
    }) {
        write(tex.into());
        bitmask |= Self::BITMASK_MY_FEATURE;
    } else {
        write(Value::SkipTexture);
    }

    // bitmask is written last
    write(bitmask.into());

    Ok(data)
}
```

### 5. Update BYTE_SIZE

Calculate the new size:
- Each `f32` or `u32`: 4 bytes
- Each texture: 20 bytes (packed format)
- Add padding to maintain alignment if needed

```rust
// Old: pub const BYTE_SIZE: usize = 224;
pub const BYTE_SIZE: usize = 244;  // 224 + 4 (factor) + 12 (color) + 20 (texture) - padding adjustment
```

**Important**: Also update the padding array in `PbrMaterialRaw` in `material.wgsl` to match.

### 6. Update WGSL Material Structs

In `material.wgsl`, mirror the Rust struct:

```wgsl
struct PbrMaterialRaw {
    // ... existing fields in exact same order as Rust writes them ...
    my_feature_factor: f32,
    my_feature_color_r: f32,
    my_feature_color_g: f32,
    my_feature_color_b: f32,
    my_feature_tex_info: TextureInfoRaw,
    // ... bitmask and padding last ...
    bitmask: u32,
    padding: array<u32, XX>  // adjust size to reach 512 bytes
};

struct PbrMaterial {
    // ... existing fields ...
    has_my_feature_texture: bool,
    my_feature_tex_info: TextureInfo,
    my_feature_factor: f32,
    my_feature_color: vec3<f32>,
}
```

Update `pbr_get_material()` to unpack the new fields:

```wgsl
fn pbr_get_material(offset: u32) -> PbrMaterial {
    const BITMASK_MY_FEATURE: u32 = 1u << 8u;  // must match Rust

    // ... in the return statement ...
    (raw.bitmask & BITMASK_MY_FEATURE) != 0u,
    convert_texture_info(raw.my_feature_tex_info),
    raw.my_feature_factor,
    vec3<f32>(raw.my_feature_color_r, raw.my_feature_color_g, raw.my_feature_color_b)
}
```

### 7. Update PbrMaterialColor

In `material_color.wgsl`:

```wgsl
struct PbrMaterialColor {
    // ... existing fields ...
    my_feature_factor: f32,
    my_feature_color: vec3<f32>,
};
```

### 8. Add Sampling Functions

See the "WGSL Sampling Patterns" section below for the three variants needed.

### 9. Update BRDF

Modify `brdf_direct()` and `brdf_ibl()` to use the new values:

```wgsl
fn brdf_direct(color: PbrMaterialColor, ...) -> vec3<f32> {
    // Use color.my_feature_factor, color.my_feature_color
    // to modify lighting calculations
}
```

---

## Immutable Properties and Shader Variants

Some material properties affect shader *structure* (bind groups, conditionals) rather than just values. These go in `PbrMaterialImmutable`:

```rust
#[derive(Clone, Debug)]
pub struct PbrMaterialImmutable {
    pub alpha_mode: MaterialAlphaMode,
    pub double_sided: bool,
    pub unlit: bool,
    // Add here if your feature needs a completely different shader
}
```

**When to use immutable properties:**
- Feature completely changes rendering approach (e.g., `unlit` skips all lighting)
- Feature requires different bind groups
- Branching would be too expensive (many conditionals in hot path)

These flow through the shader cache system:
1. `PbrMaterialImmutable` → stored in material
2. Accessed via `material.unlit()` etc.
3. Flows to `ShaderCacheKeyMaterial*` structs
4. Used in shader templates via Askama: `{% if unlit %}`

Example in `template.rs`:
```rust
pub struct ShaderTemplateTransparentMaterialFragment {
    pub unlit: bool,  // from cache_key.unlit
}
```

Used in WGSL template:
```wgsl
{% if !unlit %}
    // Full lighting calculations
{% else %}
    var color = unlit(material_color);
{% endif %}
```

---

## WGSL Sampling Patterns

The material system has three sampling contexts with different function patterns:

### Opaque Compute Shader - Gradient Mode

Uses explicit UV derivatives for anisotropic filtering:

```wgsl
// In PbrMaterialGradients struct
my_feature: UvDerivs,

// Sampling function
fn _pbr_my_feature_grad(material: PbrMaterial, attribute_uv: vec2<f32>, uv_derivs: UvDerivs) -> f32 {
    var value = material.my_feature_factor;
    if material.has_my_feature_texture {
        value *= texture_pool_sample_grad(material.my_feature_tex_info, attribute_uv, uv_derivs).a;
    }
    return value;
}
```

Also update `mipmap.wgsl` → `pbr_get_gradients()`:
```wgsl
if (material.has_my_feature_texture) {
    out.my_feature = get_uv_derivatives(..., material.my_feature_tex_info, ...);
}
```

### Opaque Compute Shader - No-Mips Mode

Samples base mip level only:

```wgsl
fn _pbr_my_feature_no_mips(material: PbrMaterial, attribute_uv: vec2<f32>) -> f32 {
    var value = material.my_feature_factor;
    if material.has_my_feature_texture {
        value *= texture_pool_sample_no_mips(material.my_feature_tex_info, attribute_uv).a;
    }
    return value;
}
```

### Transparent Fragment Shader

Uses hardware derivatives automatically:

```wgsl
fn pbr_my_feature(material: PbrMaterial, fragment_input: FragmentInput) -> f32 {
    var value = material.my_feature_factor;
    if material.has_my_feature_texture {
        let uv = texture_uv(material.my_feature_tex_info, fragment_input);
        value *= texture_pool_sample(material.my_feature_tex_info, uv).a;
    }
    return value;
}
```

### Naming Conventions

| Context | Pattern | Example |
|---------|---------|---------|
| Opaque grad | `_pbr_<name>_grad()` | `_pbr_specular_grad()` |
| Opaque no-mips | `_pbr_<name>_no_mips()` | `_pbr_specular_no_mips()` |
| Transparent | `pbr_<name>()` | `pbr_specular()` |

---

## File Locations Reference

### Rust

| File | Purpose |
|------|---------|
| `gltf/populate/material.rs` | Parse glTF extensions, populate PbrMaterial |
| `materials/pbr/material.rs` | PbrMaterial struct, BYTE_SIZE, uniform_buffer_data() |
| `materials/pbr/buffers.rs` | GPU buffer creation and upload |
| `render_passes/.../cache_key.rs` | Shader variant cache keys |
| `render_passes/.../template.rs` | Askama template structs |

### WGSL

| File | Purpose |
|------|---------|
| `shared/.../material.wgsl` | PbrMaterialRaw, PbrMaterial, pbr_get_material() |
| `shared/.../material_color.wgsl` | PbrMaterialColor struct |
| `opaque/.../material_color_calc.wgsl` | Sampling (grad + no_mips) |
| `transparent/.../material_color_calc.wgsl` | Sampling (fragment) |
| `opaque/.../mipmap.wgsl` | UV derivative computation |
| `shared/.../brdf.wgsl` | Lighting calculations |

---

## Testing Checklist

After making changes:

1. **Build** - Check for Rust and WGSL compilation errors
2. **With feature** - Test models that use the new extension
3. **Without feature** - Test models that don't use it (defaults should work)
4. **Both paths** - Verify opaque and transparent rendering
5. **Edge cases** - Test with textures and without, various factor values

---

## Example: KHR_materials_specular

This extension modifies dielectric F0 (base reflectivity):

**Rust side:**
- `specular_factor: f32` (default 1.0)
- `specular_color_factor: [f32; 3]` (default white)
- Two textures: `specular_tex` (alpha channel) and `specular_color_tex` (RGB)

**WGSL side:**
```wgsl
// In brdf.wgsl
let dielectric_f0 = min(vec3<f32>(0.04) * color.specular_color, vec3<f32>(1.0)) * color.specular;
let F0 = mix(dielectric_f0, base_color, metallic);
```

The extension is always active (uses defaults when not specified), so no shader variants needed.
