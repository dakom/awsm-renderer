# WGSL Shaders Refactor Plan

## Overview

The materials system has been refactored from a fixed 512-byte buffer approach to a flexible, variable-length buffer system. The Rust side is complete - all material data is now properly written to GPU buffers with:

- **Shader ID prefix**: First byte identifies material type (PBR=0, Unlit=1)
- **Variable-length buffers**: Each material type can have different sizes
- **Extension support**: PBR materials have 12 feature index slots pointing to optional extension data (clearcoat, sheen, etc.)

---

## What's Complete

- [x] PBR material structure & encoding (Rust)
- [x] Unlit material structure & encoding (Rust)
- [x] Feature indices system for optional extensions
- [x] All extension data packing (clearcoat, sheen, specular, transmission, volume, etc.)
- [x] Texture info packing/unpacking (5 words per texture)
- [x] Dynamic buffer system with proper offset management
- [x] `material.wgsl` - Core loading functions
- [x] `pbr_material.wgsl` - PBR material loading with on-demand extension loading
- [x] `unlit_material.wgsl` - Unlit material loading + `UnlitMaterialColor` struct
- [x] **Material Type Dispatch** - `shader_id` branching in both compute.wgsl and fragment.wgsl
- [x] **Unlit Color Computation** - `compute_unlit_material_color()` and `unlit_get_material_color()` functions
- [x] **TextureInfo.exists field** - Replaced `has_*_texture` boolean fields with `tex_info.exists`
- [x] **On-demand extension loading** - Extensions (specular, transmission, volume, etc.) loaded via indices when needed

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
|    |                        |                               |
|         <- Final rendered color ->                          |
+-------------------------------------------------------------+
```

---

## Recent Changes

### TextureInfo.exists
- `TextureInfo` struct now has an `exists` field (bit 0 in flags)
- Replaced all `has_*_texture` boolean fields on materials
- Check texture existence with `tex_info.exists` instead of separate booleans

### On-Demand Extension Loading
- Extensions are loaded using their index from `PbrMaterial`:
  - `pbr_material_load_specular(material.specular_index)`
  - `pbr_material_load_transmission(material.transmission_index)`
  - `pbr_material_load_volume(material.volume_index)`
  - etc.
- Index of 0 means feature is absent; loaders return sensible defaults

### Shader ID Dispatch
- Both `compute.wgsl` (opaque) and `fragment.wgsl` (transparent) now branch on `shader_id`:
  - `SHADER_ID_PBR` (0): Full PBR path with lighting
  - `SHADER_ID_UNLIT` (1): Simple unlit path (base_color + emissive)

---

## What Still Needs Work

### 1. Clearcoat Integration (Optional - can defer)

**Status**: Data structures complete, not integrated into lighting

**Tasks**:
1. Load clearcoat data when `pbr_material.clearcoat_index != 0`
2. Integrate into `apply_lighting()` or BRDF calculations
3. Reference: KHR_materials_clearcoat specification

---

### 2. Sheen Integration (Optional - can defer)

**Status**: Data structures complete, not integrated into lighting

**Tasks**:
1. Load sheen data when `pbr_material.sheen_index != 0`
2. Integrate into `apply_lighting()` or BRDF calculations
3. Reference: KHR_materials_sheen specification

---

### 3. Iridescence Gap (Optional - can defer)

**Problem**: WGSL loader exists but Rust writer doesn't populate it.

**Tasks**:
1. Add iridescence data writing in `pbr.rs` `uniform_buffer_data()`
2. Set `feature_indices.iridescence` to proper offset
3. Integrate into lighting calculations

---

## File Reference

### Material Loading (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `shared_wgsl/material.wgsl` | Core loading functions | Complete |
| `shared_wgsl/pbr/pbr_material.wgsl` | PBR material loading | Complete |
| `shared_wgsl/unlit/unlit_material.wgsl` | Unlit material loading + color struct | Complete |
| `shared_wgsl/lighting/unlit.wgsl` | Unlit lighting function | Complete |
| `shared_wgsl/lighting/brdf.wgsl` | PBR BRDF calculations | Needs extension integration |
| `shared_wgsl/textures.wgsl` | TextureInfo with `exists` field | Complete |

### Material Processing (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `material_opaque_wgsl/compute.wgsl` | Opaque material rendering | Complete (shader_id dispatch) |
| `material_transparent_wgsl/fragment.wgsl` | Transparent material rendering | Complete (shader_id dispatch) |
| `material_opaque_wgsl/helpers/material_color_calc.wgsl` | Color computation (both PBR and Unlit) | Complete |
| `material_transparent_wgsl/helpers/material_color_calc.wgsl` | Color computation (both PBR and Unlit) | Complete |
| `material_opaque_wgsl/helpers/mipmap.wgsl` | Gradient computation for textures | Complete |

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
```

---

## Notes

- The `material_load_shader_id(byte_offset)` function is already implemented in `material.wgsl`
- Feature indices are stored as relative offsets from base_index, converted to absolute via `abs_index()`
- When a feature index is 0, the feature is absent and loaders return sensible defaults
- Unlit materials don't have extension support by design (they're minimal)
- Texture existence is checked via `tex_info.exists` instead of separate `has_*_texture` booleans

## Extensions

### References

These references should be read in full, as they contain important details about how the extensions should be implemented and interact with each other.

- IOR: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_ior
- Transmission: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_transmission
- Volume: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_volume
- Specular: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_specular
- Clearcoat: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_clearcoat
- Sheen: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_sheen
- Unlit: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_unlit

### Test Scenes

We have these setup to test in the renderer, I can provide feedback on how they look once implemented.

- IOR: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareIor
- Also IOR: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/IORTestGrid
- Transmission: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareTransmission
- Volume: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareVolume
- Specular: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareSpecular
- Clearcoat: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareClearcoat
- Sheen: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareSheen
- Unlit: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/UnlitTest
