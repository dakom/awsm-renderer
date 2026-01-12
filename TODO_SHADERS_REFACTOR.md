# WGSL Shaders Refactor Plan

## Overview

The materials system has been refactored from a fixed 512-byte buffer approach to a flexible, variable-length buffer system. The Rust side is complete - all material data is now properly written to GPU buffers with:

- **Shader ID prefix**: First byte identifies material type (PBR=0, Unlit=1)
- **Variable-length buffers**: Each material type can have different sizes
- **Extension support**: PBR materials have 12 feature index slots pointing to optional extension data (clearcoat, sheen, etc.)

**The WGSL shaders need to be updated to properly consume this new format.**

---

## Architecture Summary

```
RUST SIDE (COMPLETE)
┌─────────────────────────────────────────────────────────────┐
│ Material enum (Pbr / Unlit)                                 │
│    ↓                                                        │
│ uniform_buffer_data() → variable-length binary blob         │
│    ↓                                                        │
│ [ShaderID][Header][Optional Features...]                    │
│    ↓                                                        │
│ GPU Material Buffer (u32 array)                             │
└─────────────────────────────────────────────────────────────┘
                         ↓
WGSL SIDE (NEEDS WORK)
┌─────────────────────────────────────────────────────────────┐
│ material_offset (byte offset from mesh metadata)            │
│    ↓                                                        │
│ material_load_shader_id() → BRANCH HERE                     │
│    ↓                        ↓                               │
│ pbr_get_material()    unlit_get_material()                  │
│    ↓                        ↓                               │
│ compute_material_color()   compute_unlit_color()            │
│    ↓                        ↓                               │
│ apply_lighting()           unlit()                          │
│    ↓                        ↓                               │
│         ← Final rendered color →                            │
└─────────────────────────────────────────────────────────────┘
```

---

## What's Working

- [x] PBR material structure & encoding (Rust)
- [x] Unlit material structure & encoding (Rust)
- [x] Feature indices system for optional extensions
- [x] Clearcoat extension data packing (18 words)
- [x] Sheen extension data packing (14 words)
- [x] All other extension data packing (specular, transmission, volume, etc.)
- [x] Texture info packing/unpacking (5 words per texture)
- [x] Dynamic buffer system with proper offset management
- [x] `material.wgsl` - Core loading functions
- [x] `pbr_material.wgsl` - PBR material loading
- [x] `unlit_material.wgsl` - Unlit material loading

---

## What Needs to be Fixed

### 1. CRITICAL: Material Type Dispatch (Shader ID Branching)

**Problem**: The shader_id is written to the buffer but **never used to branch** between PBR and Unlit rendering. All materials currently go through PBR path.

**Affected Files**:
- `crates/renderer/src/render_passes/material_opaque/shader/material_opaque_wgsl/compute.wgsl`
- `crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/fragment.wgsl`

**Current Code** (always uses PBR):
```wgsl
let pbr_material = pbr_get_material(material_offset);
// ... always does PBR lighting ...
let sample_color = apply_lighting(...);
```

**TODO comments already exist** at lines 246, 356, 459 in compute.wgsl:
```wgsl
// TODO - if material is unlit:
//let sample_color = unlit(mat_color_{{s}});
```

**Required Fix Pattern**:
```wgsl
let shader_id = material_load_shader_id(material_offset);
if (shader_id == SHADER_ID_UNLIT) {
    let unlit_material = unlit_get_material(material_offset);
    // ... compute unlit color ...
    let sample_color = unlit(unlit_color);
} else {
    let pbr_material = pbr_get_material(material_offset);
    // ... compute PBR color ...
    let sample_color = apply_lighting(...);
}
```

---

### 2. Unlit Color Computation

**Problem**: There's no `compute_material_color_unlit()` function equivalent to PBR's `compute_material_color()`.

**The `unlit()` function exists** in `shared_wgsl/lighting/unlit.wgsl`:
```wgsl
fn unlit(color: PbrMaterialColor) -> vec3<f32> {
    return color.base.rgb + color.emissive;
}
```

**Tasks**:
1. Either create a dedicated `compute_unlit_color()` function, or
2. Adapt the existing code to work with UnlitMaterial struct
3. The unlit path only needs: base_color (texture + factor) + emissive (texture + factor) + vertex color

---

### 3. Clearcoat Integration (Optional - can defer)

**Status**: Data structures complete, not integrated into lighting

**Rust side** (complete):
- `PbrMaterialClearCoat` struct in `materials/pbr.rs`
- 18 words: tex(5) + factor(1) + roughness_tex(5) + roughness_factor(1) + normal_tex(5) + normal_scale(1)

**WGSL side** (loader exists, not used):
- `pbr_material_load_clearcoat()` in `pbr_material.wgsl` lines 340-359
- **Not called from any lighting function**

**Tasks**:
1. Load clearcoat data when `pbr_material.clearcoat_index != 0`
2. Integrate into `apply_lighting()` or BRDF calculations
3. Reference: KHR_materials_clearcoat specification

---

### 4. Sheen Integration (Optional - can defer)

**Status**: Data structures complete, not integrated into lighting

**Rust side** (complete):
- `PbrMaterialSheen` struct in `materials/pbr.rs`
- 14 words: roughness_tex(5) + roughness_factor(1) + color_tex(5) + color_factor(3)

**WGSL side** (loader exists, not used):
- `pbr_material_load_sheen()` in `pbr_material.wgsl` lines 370-384
- **Not called from any lighting function**

**Tasks**:
1. Load sheen data when `pbr_material.sheen_index != 0`
2. Integrate into `apply_lighting()` or BRDF calculations
3. Reference: KHR_materials_sheen specification

---

### 5. Iridescence Gap (Optional - can defer)

**Problem**: WGSL loader exists but Rust writer doesn't populate it.

**WGSL** (complete):
- `pbr_material_load_iridescence()` in `pbr_material.wgsl` lines 406-440
- 14 words: tex(5) + factor(1) + ior(1) + thickness_tex(5) + thickness_min(1) + thickness_max(1)

**Rust** (incomplete):
- `PbrMaterialIridescence` struct exists but `feature_indices.iridescence` is never set
- Comment in code acknowledges this gap

**Tasks**:
1. Add iridescence data writing in `pbr.rs` `uniform_buffer_data()`
2. Set `feature_indices.iridescence` to proper offset
3. Integrate into lighting calculations

---

## Implementation Order

### Phase 1: Core Material Type Dispatch (Required)

This is the critical fix to make the new system actually work:

1. **Update compute.wgsl** (opaque materials)
   - Add shader_id check at the start of material processing
   - Branch to unlit path when `shader_id == SHADER_ID_UNLIT`
   - Apply `unlit()` function instead of `apply_lighting()` for unlit materials
   - Fix all 3 TODO locations (lines ~246, ~356, ~459)

2. **Update fragment.wgsl** (transparent materials)
   - Same changes as compute.wgsl
   - Add shader_id branching
   - Route unlit materials to `unlit()` function

3. **Create/adapt unlit color computation**
   - Either a new helper function or inline adaptation
   - Only needs: base_color_tex, base_color_factor, emissive_tex, emissive_factor
   - Plus vertex color handling if applicable

### Phase 2: Extension Integration (Can Defer)

These can be done incrementally after Phase 1:

4. **Clearcoat integration**
   - Call `pbr_material_load_clearcoat()` when index != 0
   - Add clearcoat BRDF contribution to lighting

5. **Sheen integration**
   - Call `pbr_material_load_sheen()` when index != 0
   - Add sheen BRDF contribution to lighting

6. **Iridescence completion**
   - Complete Rust-side writing
   - Integrate into lighting calculations

---

## File Reference

### Material Loading (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `shared_wgsl/material.wgsl` | Core loading functions | Complete |
| `shared_wgsl/pbr/pbr_material.wgsl` | PBR material loading | Complete |
| `shared_wgsl/unlit/unlit_material.wgsl` | Unlit material loading | Complete |
| `shared_wgsl/lighting/unlit.wgsl` | Unlit lighting function | Complete |
| `shared_wgsl/lighting/brdf.wgsl` | PBR BRDF calculations | Needs extension integration |

### Material Processing (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `material_opaque_wgsl/compute.wgsl` | Opaque material rendering | **Needs shader_id dispatch** |
| `material_transparent_wgsl/fragment.wgsl` | Transparent material rendering | **Needs shader_id dispatch** |
| `material_opaque_wgsl/bind_groups.wgsl` | Binding declarations | OK |
| `material_transparent_wgsl/bind_groups.wgsl` | Binding declarations | OK |

### Material Color Computation (WGSL)
| File | Purpose | Status |
|------|---------|--------|
| `shared_wgsl/pbr/pbr_material_color.wgsl` | PBR color computation | Complete |
| (missing) | Unlit color computation | **Needs creation or adaptation** |

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
