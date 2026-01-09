# Materials: Adding New Shader Features

This document explains how to add new material features (like glTF extensions) to the renderer shader system. The architecture supports both opaque (compute shader) and transparent (fragment shader) rendering paths.

## Overview

The material system has several key components:

1. **Material struct** (`material.wgsl`) - Defines material properties and texture references
2. **Material color struct** (`material_color.wgsl`) - Holds sampled values for BRDF calculations
3. **Material sampling** (`material_color_calc.wgsl`) - Samples textures and computes material properties
4. **BRDF calculations** (`brdf.wgsl`) - Uses material values for lighting
5. **Gradient computation** (`mipmap.wgsl`) - Computes UV derivatives for compute shader path

## File Locations

```
crates/renderer/src/render_passes/
├── shared/shared_wgsl/pbr/
│   ├── material.wgsl           # PbrMaterial struct (shared)
│   ├── material_color.wgsl     # PbrMaterialColor struct (shared)
│   └── lighting/
│       └── brdf.wgsl           # BRDF functions (shared)
├── material_opaque/shader/material_opaque_wgsl/helpers/
│   ├── material_color_calc.wgsl  # Opaque sampling (grad + no_mips variants)
│   └── mipmap.wgsl               # UV derivative computation
└── material_transparent/shader/material_transparent_wgsl/helpers/
    └── material_color_calc.wgsl  # Transparent sampling (fragment shader)
```

## Step-by-Step: Adding a New Material Feature

### 1. Update PbrMaterialColor Struct

Add your sampled values to `material_color.wgsl`:

```wgsl
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
    // Add your new fields here:
    my_feature_factor: f32,
    my_feature_color: vec3<f32>,
};
```

### 2. Add Sampling Functions (Opaque Shader)

The opaque shader (`material_opaque/.../material_color_calc.wgsl`) has TWO variants controlled by templates:

#### Gradient Mode (MipmapMode::Gradient)

Add to `PbrMaterialGradients` struct:
```wgsl
struct PbrMaterialGradients {
    // ... existing fields ...
    my_feature: UvDerivs,
}
```

Add sampling function:
```wgsl
fn _pbr_my_feature_grad(material: PbrMaterial, attribute_uv: vec2<f32>, uv_derivs: UvDerivs) -> f32 {
    var value = material.my_feature_factor;
    if material.has_my_feature_texture {
        value *= texture_pool_sample_grad(material.my_feature_tex_info, attribute_uv, uv_derivs).a;
    }
    return value;
}
```

Update `pbr_get_material_color_grad()` to call your function and include in return.

#### No-Mips Mode (MipmapMode::None)

Add the same pattern without gradients:
```wgsl
fn _pbr_my_feature_no_mips(material: PbrMaterial, attribute_uv: vec2<f32>) -> f32 {
    var value = material.my_feature_factor;
    if material.has_my_feature_texture {
        value *= texture_pool_sample_no_mips(material.my_feature_tex_info, attribute_uv).a;
    }
    return value;
}
```

Update `pbr_get_material_color_no_mips()` to call your function and include in return.

### 3. Add Sampling Functions (Transparent Shader)

The transparent shader (`material_transparent/.../material_color_calc.wgsl`) uses hardware derivatives:

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

Update `pbr_get_material_color()` to call your function and include in return.

### 4. Update Gradient Computation (mipmap.wgsl)

Add gradient computation in `pbr_get_gradients()`:

```wgsl
if (material.has_my_feature_texture) {
    out.my_feature = get_uv_derivatives(
        barycentric,
        bary_derivs,
        triangle_indices,
        attribute_data_offset, vertex_attribute_stride,
        uv_sets_index,
        material.my_feature_tex_info,
        world_normal,
        view_matrix
    );
}
```

### 5. Update BRDF Functions (brdf.wgsl)

Modify `brdf_direct()` and `brdf_ibl()` to use your new material values:

```wgsl
fn brdf_direct(color: PbrMaterialColor, ...) -> vec3<f32> {
    // Use color.my_feature_factor, color.my_feature_color, etc.
    // to modify the lighting calculation
}
```

## Naming Conventions

| Path | Naming Pattern | Example |
|------|---------------|---------|
| Opaque grad | `_pbr_<name>_grad()` | `_pbr_specular_grad()` |
| Opaque no-mips | `_pbr_<name>_no_mips()` | `_pbr_specular_no_mips()` |
| Transparent | `pbr_<name>()` | `pbr_specular()` |

The underscore prefix indicates private/internal functions.

## Texture Sampling Functions

Different contexts require different sampling approaches:

| Context | Function | Notes |
|---------|----------|-------|
| Compute (gradient) | `texture_pool_sample_grad(tex_info, uv, uv_derivs)` | Explicit derivatives |
| Compute (no-mips) | `texture_pool_sample_no_mips(tex_info, uv)` | Base mip only |
| Fragment | `texture_pool_sample(tex_info, uv)` | Hardware derivatives |

## Early Exit Patterns

For optional features, use early returns when textures are absent:

```wgsl
fn _pbr_my_feature_grad(material: PbrMaterial, ...) -> f32 {
    // Default value when feature is not used
    var value = material.my_feature_factor;

    // Only sample if texture exists
    if material.has_my_feature_texture {
        value *= texture_pool_sample_grad(...).a;
    }

    return value;
}
```

For features that can be completely disabled (e.g., specular_factor = 0), the BRDF functions should handle the degenerate case efficiently.

## Example: KHR_materials_specular

The specular extension modifies F0 (base reflectivity) for dielectrics:

```wgsl
// In brdf.wgsl
// Standard: F0 = 0.04 for dielectrics
// With extension: dielectric_f0 = min(0.04 * specular_color, 1.0) * specular

let dielectric_f0 = min(vec3<f32>(0.04) * color.specular_color, vec3<f32>(1.0)) * color.specular;
let F0 = mix(dielectric_f0, base_color, metallic);
```

Key values:
- `specular` (f32): Strength factor, default 1.0 (from specularTexture.a * specularFactor)
- `specular_color` (vec3): F0 color modifier, default white (from specularColorTexture.rgb * specularColorFactor)

## Rust Side (Not Covered Here)

This document focuses on shader changes. On the Rust side, you also need to:

1. Update `PbrMaterial` struct in Rust
2. Add bitmask constants for texture presence flags
3. Update material buffer serialization
4. Handle texture loading for the new feature

See the existing material implementation for reference patterns.

## Testing

After making shader changes:

1. Build the project to check for WGSL compilation errors
2. Test with models that use the new feature
3. Test with models that don't use it (ensure defaults work)
4. Verify both opaque and transparent rendering paths
