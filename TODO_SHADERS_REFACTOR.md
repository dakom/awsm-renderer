# WGSL Shaders Refactor Plan

## Overview

The materials system has been refactored from a fixed 512-byte buffer approach to a flexible, variable-length buffer system. The Rust side is complete - all material data is now properly written to GPU buffers with:

- **Shader ID prefix**: First byte identifies material type (PBR=0, Unlit=1)
- **Variable-length buffers**: Each material type can have different sizes
- **Extension support**: PBR materials have 12 feature index slots pointing to optional extension data

---

## What's Complete

### Core Material System
- [x] PBR material structure & encoding (Rust)
- [x] Unlit material structure & encoding (Rust)
- [x] Feature indices system for optional extensions
- [x] Texture info packing/unpacking (5 words per texture)
- [x] Dynamic buffer system with proper offset management
- [x] `material.wgsl` - Core loading functions
- [x] `pbr_material.wgsl` - PBR material loading with on-demand extension loading
- [x] `unlit_material.wgsl` - Unlit material loading + `UnlitMaterialColor` struct
- [x] **Material Type Dispatch** - `shader_id` branching in both compute.wgsl and fragment.wgsl
- [x] **Unlit Color Computation** - `compute_unlit_material_color()` and `unlit_get_material_color()` functions
- [x] **TextureInfo.exists field** - Replaced `has_*_texture` boolean fields with `tex_info.exists`
- [x] **On-demand extension loading** - Extensions loaded via indices when needed

### Implemented Extensions
- [x] **KHR_materials_ior** - Index of refraction
- [x] **KHR_materials_specular** - Specular factor and color
- [x] **KHR_materials_transmission** - Light transmission through surfaces
- [x] **KHR_materials_volume** - Volume attenuation (Beer's Law)
- [x] **KHR_materials_emissive_strength** - HDR emissive values
- [x] **KHR_materials_clearcoat** - Clear coating layer with own normal/roughness
- [x] **KHR_materials_sheen** - Cloth-like sheen at grazing angles (Charlie distribution)
- [x] **KHR_materials_unlit** - Unlit materials

---

## Architecture Summary

```
RUST SIDE (COMPLETE)
+-------------------------------------------------------------+
| Material enum (Pbr / Unlit)                                 |
|    |                                                        |
| uniform_buffer_data() -> variable-length binary blob        |
|    |                                                        |
| [ShaderID][Header][Optional Features...]                    |
|    |                                                        |
| GPU Material Buffer (u32 array)                             |
+-------------------------------------------------------------+
                         |
WGSL SIDE (COMPLETE)
+-------------------------------------------------------------+
| material_offset (byte offset from mesh metadata)            |
|    |                                                        |
| material_load_shader_id() -> BRANCH                         |
|    |                        |                               |
| pbr_get_material()    unlit_get_material()                  |
|    |                        |                               |
| compute_material_color()   compute_unlit_material_color()   |
|    |                        |                               |
| apply_lighting()           compute_unlit_output()           |
|   (with clearcoat/sheen)                                    |
|    |                        |                               |
|         <- Final rendered color ->                          |
+-------------------------------------------------------------+
```

---

## What Still Needs Work

### 1. Anisotropy Integration (Optional - can defer)

**Status**: Rust data structures complete, WGSL loader exists, not integrated into BRDF

**Tasks**:
1. Load anisotropy data when `pbr_material.anisotropy_index != 0`
2. Integrate into `brdf_direct()` and `brdf_ibl()` - modify GGX distribution for anisotropic highlights
3. Sample anisotropy texture to get direction/strength
4. Reference: KHR_materials_anisotropy specification

---

### 2. Dispersion Integration (Optional - can defer)

**Status**: Rust data structures complete, WGSL loader exists, not integrated

**Tasks**:
1. Load dispersion data when `pbr_material.dispersion_index != 0`
2. Implement chromatic dispersion for transmitted light
3. Split transmission into RGB wavelengths with different IOR offsets
4. Reference: KHR_materials_dispersion specification

---

### 3. Iridescence Gap (Optional - can defer)

**Status**: WGSL loader exists but Rust writer doesn't populate it

**Tasks**:
1. Add iridescence data writing in `pbr.rs` `uniform_buffer_data()`
2. Set `feature_indices.iridescence` to proper offset
3. Implement thin-film interference calculation in BRDF
4. Requires iridescence factor, IOR, thickness, and optional thickness texture
5. Reference: KHR_materials_iridescence specification

---

### 4. Diffuse Transmission (Optional - can defer)

**Status**: Rust data structures complete, WGSL loader exists, not integrated

**Tasks**:
1. Load diffuse transmission data when `pbr_material.diffuse_transmission_index != 0`
2. Implement subsurface scattering approximation
3. Blend diffuse transmission with regular transmission
4. Reference: KHR_materials_diffuse_transmission specification

---

## File Reference

### Material Loading (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `shared_wgsl/material.wgsl` | Core loading functions | Complete |
| `shared_wgsl/pbr/pbr_material.wgsl` | PBR material loading | Complete |
| `shared_wgsl/pbr/pbr_material_color.wgsl` | PbrMaterialColor struct | Complete (with clearcoat/sheen) |
| `shared_wgsl/unlit/unlit_material.wgsl` | Unlit material loading + color struct | Complete |
| `shared_wgsl/lighting/unlit.wgsl` | Unlit lighting function | Complete |
| `shared_wgsl/lighting/brdf.wgsl` | PBR BRDF calculations | Complete (with clearcoat/sheen) |
| `shared_wgsl/textures.wgsl` | TextureInfo with `exists` field | Complete |

### Material Processing (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `material_opaque_wgsl/compute.wgsl` | Opaque material rendering | Complete |
| `material_transparent_wgsl/fragment.wgsl` | Transparent material rendering | Complete |
| `material_opaque_wgsl/helpers/material_color_calc.wgsl` | Color computation (opaque) | Complete (with clearcoat/sheen) |
| `material_transparent_wgsl/helpers/material_color_calc.wgsl` | Color computation (transparent) | Complete (with clearcoat/sheen) |
| `material_opaque_wgsl/helpers/mipmap.wgsl` | Gradient computation for textures | Complete (with clearcoat/sheen) |
| `material_opaque_wgsl/shader/template.rs` | Askama template context | Complete (with DRY helpers) |

---

## Constants Reference

```wgsl
// material.wgsl
const SHADER_ID_PBR: u32 = 0u;
const SHADER_ID_UNLIT: u32 = 1u;

// pbr_material.wgsl
const PBR_CORE_WORDS: u32 = 38u;
const PBR_FEATURE_INDEX_WORDS: u32 = 12u;
const PBR_HEADER_WORDS: u32 = 50u;

// brdf.wgsl
const CLEARCOAT_F0: f32 = 0.04;  // Dielectric F0 for clearcoat
```

---

## Extension Implementation Summary

### Clearcoat (KHR_materials_clearcoat)
- **PbrMaterialColor fields**: `clearcoat`, `clearcoat_roughness`, `clearcoat_normal`
- **BRDF**: Separate GGX specular layer on top with F0=0.04
- **Energy conservation**: Base layer attenuated by `(1 - clearcoat * Fc)`

### Sheen (KHR_materials_sheen)
- **PbrMaterialColor fields**: `sheen_color`, `sheen_roughness`
- **BRDF**: Charlie distribution (cloth-like sheen at grazing angles)
- **Energy conservation**: Base layer scaled by `sheen_albedo_scaling()`

---

## Notes

- The `material_load_shader_id(byte_offset)` function is implemented in `material.wgsl`
- Feature indices are stored as relative offsets, converted to absolute via `abs_index()`
- When a feature index is 0, the feature is absent and loaders return sensible defaults
- Unlit materials don't have extension support by design (they're minimal)
- Texture existence is checked via `tex_info.exists` instead of separate `has_*_texture` booleans
- Template DRY: `MipmapMode` has helper methods (`suffix()`, `sample_fn()`, `is_gradient()`) to reduce duplication

---

## Extensions

### References

These references should be read in full for proper implementation:

- IOR: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_ior
- Transmission: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_transmission
- Volume: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_volume
- Specular: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_specular
- Clearcoat: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_clearcoat
- Sheen: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_sheen
- Unlit: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_unlit
- Anisotropy: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_anisotropy
- Dispersion: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_dispersion
- Iridescence: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_iridescence
- Diffuse Transmission: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_diffuse_transmission

### Test Scenes

Available in the renderer for testing:

- IOR: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareIor
- Also IOR: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/IORTestGrid
- Transmission: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareTransmission
- Volume: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareVolume
- Specular: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareSpecular
- Clearcoat: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareClearcoat
- Sheen: https://github.com/KhronosGroup/glTF/tree/main/Models/CompareSheen
- Unlit: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/UnlitTest
