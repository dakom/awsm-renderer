## Goal

1. Implement the IOR, transmission, and volume extensions
2. Update the specular extension (and any other code paths) to account for IOR

## Preparation Already Completed

Everything is available in `PbrMaterial` in `material.wgsl`. The struct already contains all necessary fields:
- `ior: f32`
- `transmission_factor: f32`
- `transmission_tex_info: TextureInfo`
- `volume_thickness_factor: f32`
- `volume_thickness_tex_info: TextureInfo`
- `volume_attenuation_distance: f32`
- `volume_attenuation_color: vec3<f32>`

No further changes needed on the Rust side - all changes are confined to shader code.

---

## Prerequisites & Architectural Decisions

**These decisions should be made BEFORE implementation begins.**

### 1. Transmission Background Sampling Strategy

Transmission requires sampling "what's behind" the surface. Choose an approach:

| Option | Quality | Cost | Requirements |
|--------|---------|------|--------------|
| **IBL-only fallback** | Low | Minimal | None - uses existing IBL |
| **Screen-space refraction** | Medium | Medium | Color buffer from previous pass |
| **Order-Independent Transparency (OIT)** | High | High | Major pipeline restructure |

**Recommendation:** Start with IBL-only fallback. If screen-space refraction is desired later:
- Render opaque objects first to a color buffer
- Transmissive objects sample from this buffer with UV offset based on IOR/roughness
- This requires Rust-side changes to pass the color buffer texture to the transparent pass

**Decision needed:** Which approach to use? IBL-only is sufficient for initial implementation.

### 2. Thin-Walled vs Volumetric Surfaces

When `volume_thickness_factor = 0` (default):
- Surface is treated as infinitely thin
- No macroscopic refraction (light passes straight through)
- Only Fresnel-based transmission/reflection split applies

When `volume_thickness_factor > 0`:
- Light refracts according to IOR (Snell's law)
- Beer's Law attenuation applies based on distance through medium
- Requires closed/manifold mesh for accurate results

**Decision needed:** None - this is spec behavior, just be aware of the distinction.

### 3. Outside IOR Assumption

The spec assumes outside IOR = 1.0 (air). This is hardcoded and not configurable.

### 4. Total Internal Reflection (TIR)

When light hits a surface at a steep angle from inside a dense medium, it reflects instead of refracting. The implementation must handle this gracefully (fall back to reflection when `sin²θt > 1`).

### 5. Alpha vs Transmission Independence

Per spec: `transmissionFactor` and `baseColor.a` operate independently:
- Alpha affects surface existence (coverage)
- Transmission affects light behavior through existing surface
- Both can be used simultaneously

---

## Optimization Considerations

Apply these patterns throughout implementation:

### 1. Bitmask Early-Exit for Texture Sampling

The `PbrMaterial` struct uses a bitmask to indicate which textures are present. Always check `has_*_texture` before sampling:

```wgsl
// GOOD - early exit avoids texture sample
fn _pbr_transmission(material: PbrMaterial, attribute_uv: vec2<f32>) -> f32 {
    // Early exit: if no texture and factor is 0, skip entirely
    if (!material.has_transmission_texture && material.transmission_factor == 0.0) {
        return 0.0;
    }

    var transmission = material.transmission_factor;
    if (material.has_transmission_texture) {
        transmission *= texture_sample(...).r;
    }
    return transmission;
}
```

### 2. Skip Transmission/Volume Processing for Non-Transmissive Materials

In BRDF functions, early-exit when transmission is 0:

```wgsl
// In brdf_direct/brdf_ibl:
// If no transmission, use standard diffuse path (cheaper)
if (color.transmission == 0.0) {
    // existing diffuse + specular code, no BTDF
    return standard_brdf_result;
}
// Otherwise, compute full transmission BTDF...
```

### 3. Skip Volume Attenuation When Not Needed

```wgsl
// Skip attenuation calculation when:
// - thickness is 0 (thin-walled)
// - attenuation_distance is infinite (no absorption)
// - attenuation_color is white (no color shift)
fn should_apply_volume_attenuation(color: PbrMaterialColor) -> bool {
    return color.volume_thickness > 0.0
        && color.volume_attenuation_distance < 1e10  // not "infinite"
        && any(color.volume_attenuation_color < vec3<f32>(1.0));
}
```

### 4. Metallic Materials Don't Transmit

Metals absorb all transmitted light. Skip transmission entirely for metallic=1.0:

```wgsl
// transmission is only visible for dielectrics
let effective_transmission = color.transmission * (1.0 - metallic);
if (effective_transmission == 0.0) {
    // skip BTDF entirely
}
```

### 5. IOR=1.0 Optimization

When IOR equals 1.0 (air-to-air), there's no refraction - light passes straight through:

```wgsl
fn refract_direction(incident: vec3<f32>, normal: vec3<f32>, ior: f32) -> vec3<f32> {
    // No refraction when IOR = 1.0
    if (ior == 1.0) {
        return incident;
    }
    // ... full Snell's law calculation
}
```

### 6. Precompute IOR-derived Values

The `ior_to_f0` calculation can be done once per material, not per-light:

```wgsl
// In brdf functions, compute once at the top:
let dielectric_f0_base = ior_to_f0(color.ior);
// Then reuse for all light calculations
```

---

## General Considerations

Both opaque and transparent materials can have these properties. The logic for handling these properties should be integrated into the existing material system without disrupting the current functionality.

If it helps, refactoring code into `shared_wgsl/*` is allowed to keep things clean and modular.

Call-out any assumptions you make or decisions you take that might affect other parts of the codebase.

Some existing code, particularly `specular` extension code, will be affected by these new changes, especially IOR.

## References

These references should be read in full, as they contain important details about how the extensions should be implemented and interact with each other.

- IOR: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_ior
- Transmission: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_transmission
- Volume: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_volume
- Specular: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_specular

## Test Scenes

We have these setup to test in the renderer, I can provide feedback on how they look once implemented.

- IOR: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareIor
- Transmission: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareTransmission
- Volume: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareVolume
- Specular: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareSpecular

---

## Implementation Plan

### Phase 1: IOR Foundation + Specular Update

**Goal:** Replace hardcoded 0.04 with dynamic IOR-based F0 calculation.

#### Key Formula
```
dielectric_f0_from_ior = ((ior - 1) / (ior + 1))^2
```
The default IOR of 1.5 yields 0.04 (current hardcoded value).

#### Files to Modify

**1. `shared_wgsl/pbr/material_color.wgsl`**
- Add `ior: f32` field to `PbrMaterialColor` struct
- The IOR value comes directly from `PbrMaterial.ior` (no texture)

**2. `material_opaque_wgsl/helpers/material_color_calc.wgsl`**
- In `pbr_get_material_color_grad()`: Pass `material.ior` to `PbrMaterialColor`
- In `pbr_get_material_color_no_mips()`: Pass `material.ior` to `PbrMaterialColor`

**3. `material_transparent_wgsl/helpers/material_color_calc.wgsl`**
- In `pbr_get_material_color()`: Pass `material.ior` to `PbrMaterialColor`

**4. `shared_wgsl/pbr/lighting/brdf.wgsl`**
- Add helper function for IOR to F0 conversion:
  ```wgsl
  fn ior_to_f0(ior: f32) -> f32 {
      // Handle IOR=0 backwards compatibility mode (Fresnel = 1.0)
      if (ior == 0.0) {
          return 1.0;
      }
      let ratio = (ior - 1.0) / (ior + 1.0);
      return ratio * ratio;
  }
  ```
- In `brdf_direct()`:
  - Replace `vec3<f32>(0.04)` with `vec3<f32>(ior_to_f0(color.ior))`
  - Handle IOR=0 backwards compatibility mode (f90 = 1.0 for all angles)
- In `brdf_ibl()`:
  - Same changes as `brdf_direct()`

#### Updated Specular Formula
With IOR, the dielectric F0 calculation becomes:
```wgsl
let dielectric_f0_base = ior_to_f0(color.ior);
let dielectric_f0 = min(vec3<f32>(dielectric_f0_base) * color.specular_color, vec3<f32>(1.0)) * color.specular;
```

#### IOR Backwards Compatibility Mode (IOR = 0)
When `ior == 0.0`:
- Fresnel term evaluates to 1.0 regardless of view direction
- This is for legacy spec-gloss material conversion
- Implementation: skip Fresnel calculation, use F = vec3(1.0)

---

### Phase 2: Transmission Extension

**Goal:** Allow light to pass through surfaces (like glass, water) instead of being diffusely scattered.

#### Key Concepts
- `transmission` (0-1): Percentage of light transmitted through surface
- Metallic surfaces (metallic=1.0) absorb transmitted light - no transmission visible
- Transmitted light is tinted by `baseColor`
- Surface roughness blurs transmitted content (microfacet refraction)

#### Core BRDF Modification
From the spec:
```
dielectric_brdf = fresnel_mix(
    base = mix(diffuse_brdf, specular_btdf * baseColor, transmission),
    layer = specular_brdf
)
```

#### Files to Modify

**1. `shared_wgsl/pbr/material_color.wgsl`**
- Add `transmission: f32` field to `PbrMaterialColor`

**2. `material_opaque_wgsl/helpers/material_color_calc.wgsl` & `material_transparent_wgsl/helpers/material_color_calc.wgsl`**
- Add transmission texture sampling functions:
  ```wgsl
  fn _pbr_transmission_grad(material: PbrMaterial, attribute_uv: vec2<f32>, uv_derivs: UvDerivs) -> f32 {
      // Early exit optimization
      if (!material.has_transmission_texture && material.transmission_factor == 0.0) {
          return 0.0;
      }
      var transmission = material.transmission_factor;
      if (material.has_transmission_texture) {
          transmission *= texture_pool_sample_grad(material.transmission_tex_info, attribute_uv, uv_derivs).r;
      }
      return transmission;
  }
  ```
- Add `transmission: UvDerivs` to `PbrMaterialGradients` struct (gradient version)
- Pass transmission to `PbrMaterialColor` construction

**3. `shared_wgsl/pbr/lighting/brdf.wgsl`**
- Add microfacet BTDF function for transmission:
  ```wgsl
  fn btdf_transmission(
      n: vec3<f32>,
      v: vec3<f32>,
      roughness: f32,
      ior: f32,
      transmission: f32,
      base_color: vec3<f32>,
      // Environment/background sample...
  ) -> vec3<f32>
  ```
- In `brdf_direct()`:
  - Early exit if `transmission == 0.0` (optimization)
  - Blend between diffuse and BTDF based on transmission factor
  - `base = mix(diffuse_brdf, btdf * base_color, transmission)`
  - Metals don't transmit: scale by `(1.0 - metallic)`
- In `brdf_ibl()`:
  - Early exit if `transmission == 0.0` (optimization)
  - Sample environment in refracted direction (or blurred based on roughness)
  - Apply same mixing logic

#### Transmission Sampling (IBL Fallback)

For initial implementation, sample the IBL environment map in the view direction (or refracted direction if volume is present):

```wgsl
// Simple IBL-based transmission (no screen-space)
fn sample_transmission_ibl(
    v: vec3<f32>,
    n: vec3<f32>,
    roughness: f32,
    ior: f32,
    has_volume: bool,
    ibl_tex: texture_cube<f32>,
    ibl_sampler: sampler,
    ibl_info: IblInfo
) -> vec3<f32> {
    var sample_dir = -v;  // Default: straight through

    // If volumetric, apply refraction
    if (has_volume && ior != 1.0) {
        sample_dir = refract_direction(v, n, 1.0 / ior);
        // Handle TIR - fall back to reflection
        if (length(sample_dir) < 0.001) {
            sample_dir = reflect(-v, n);
        }
    }

    // Sample with roughness-based blur
    return samplePrefilteredEnv(sample_dir, roughness, ibl_tex, ibl_sampler, ibl_info);
}
```

---

### Phase 3: Volume Extension

**Goal:** Add realistic light absorption and refraction for solid objects with thickness.

#### Key Concepts
- `thicknessFactor` (default 0): Distance light travels through medium (mesh-local units)
- `attenuationDistance` (default +∞): Distance at which light reaches `attenuationColor`
- `attenuationColor` (default white): Color light becomes at attenuation distance
- Requires closed/manifold mesh for accurate results
- Only meaningful when combined with transmission

#### Attenuation Formula (Beer's Law)
```wgsl
fn volume_attenuation(distance: f32, attenuation_color: vec3<f32>, attenuation_distance: f32) -> vec3<f32> {
    // Early exit optimizations
    if (distance <= 0.0) {
        return vec3<f32>(1.0);  // No distance = no attenuation
    }
    if (attenuation_distance <= 0.0 || attenuation_distance > 1e10) {
        return vec3<f32>(1.0);  // Infinite distance = no attenuation
    }
    if (all(attenuation_color >= vec3<f32>(0.999))) {
        return vec3<f32>(1.0);  // White = no color shift
    }

    // Beer's Law: T(x) = c^(x/d)
    return pow(attenuation_color, vec3<f32>(distance / attenuation_distance));
}
```

#### Refraction
Volume uses IOR for actual light bending (macroscopic refraction), not just Fresnel.
```wgsl
// Refracted direction (Snell's law)
fn refract_direction(incident: vec3<f32>, normal: vec3<f32>, eta: f32) -> vec3<f32> {
    // Optimization: no refraction when eta = 1.0 (same medium)
    if (abs(eta - 1.0) < 0.001) {
        return incident;
    }

    // eta = ior_outside / ior_inside (typically 1.0 / ior for entering)
    let cos_i = -dot(incident, normal);
    let sin_t2 = eta * eta * (1.0 - cos_i * cos_i);

    // Total internal reflection
    if (sin_t2 > 1.0) {
        return vec3<f32>(0.0);  // Signal TIR to caller
    }

    let cos_t = sqrt(1.0 - sin_t2);
    return eta * incident + (eta * cos_i - cos_t) * normal;
}
```

#### Files to Modify

**1. `shared_wgsl/pbr/material_color.wgsl`**
- Add to `PbrMaterialColor`:
  ```wgsl
  volume_thickness: f32,
  volume_attenuation_distance: f32,
  volume_attenuation_color: vec3<f32>,
  ```

**2. `material_*_wgsl/helpers/material_color_calc.wgsl`**
- Add thickness texture sampling:
  ```wgsl
  fn _pbr_volume_thickness_grad(material: PbrMaterial, attribute_uv: vec2<f32>, uv_derivs: UvDerivs) -> f32 {
      // Early exit: no volume if thickness is 0 and no texture
      if (!material.has_volume_thickness_texture && material.volume_thickness_factor == 0.0) {
          return 0.0;
      }
      var thickness = material.volume_thickness_factor;
      if (material.has_volume_thickness_texture) {
          thickness *= texture_pool_sample_grad(material.volume_thickness_tex_info, attribute_uv, uv_derivs).g;
      }
      return thickness;
  }
  ```
- Add `volume_thickness: UvDerivs` to `PbrMaterialGradients` struct
- Pass all volume properties to `PbrMaterialColor`

**3. `shared_wgsl/pbr/lighting/brdf.wgsl`** (or new file `shared_wgsl/pbr/volume.wgsl`)
- Add `volume_attenuation()` function
- Add `refract_direction()` helper function
- Modify transmission code to:
  1. Check if volume is active (`thickness > 0`)
  2. Calculate refracted ray direction using IOR
  3. Apply Beer's Law attenuation to transmitted color

#### Volume + Transmission Integration
When both are present:
```wgsl
// In BTDF calculation:
let has_volume = color.volume_thickness > 0.0;

// Get transmission sample direction
var sample_dir = -v;
if (has_volume && color.ior != 1.0) {
    let refracted = refract_direction(v, n, 1.0 / color.ior);
    if (length(refracted) > 0.001) {
        sample_dir = refracted;
    }
    // else: TIR, keep original direction or reflect
}

let transmitted_color = sample_environment(sample_dir, roughness);

// Apply volume attenuation if active
var attenuation = vec3<f32>(1.0);
if (has_volume) {
    attenuation = volume_attenuation(
        color.volume_thickness,
        color.volume_attenuation_color,
        color.volume_attenuation_distance
    );
}

let final_transmission = transmitted_color * base_color * attenuation;
```

---

### Summary of `PbrMaterialColor` Changes

**Current struct:**
```wgsl
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
    specular: f32,
    specular_color: vec3<f32>,
};
```

**Updated struct:**
```wgsl
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
    specular: f32,
    specular_color: vec3<f32>,
    // New fields for extensions:
    ior: f32,
    transmission: f32,
    volume_thickness: f32,
    volume_attenuation_distance: f32,
    volume_attenuation_color: vec3<f32>,
};
```

---

### File Change Summary

| File | Changes |
|------|---------|
| `shared_wgsl/pbr/material_color.wgsl` | Add ior, transmission, volume fields to struct |
| `shared_wgsl/pbr/lighting/brdf.wgsl` | Add `ior_to_f0()`, update F0 calculations, add BTDF, add volume attenuation, add early-exit optimizations |
| `material_opaque_wgsl/helpers/material_color_calc.wgsl` | Add transmission/volume texture sampling with early-exit, update struct construction, update `PbrMaterialGradients` |
| `material_transparent_wgsl/helpers/material_color_calc.wgsl` | Same as opaque |

Optional new file:
| `shared_wgsl/pbr/volume.wgsl` | Volume-specific functions (attenuation, refraction) if brdf.wgsl gets too large |

---

### Implementation Order

1. **IOR + Specular Update** (Phase 1)
   - Smallest change, foundational for other extensions
   - Test with CompareIor and CompareSpecular scenes

2. **Transmission** (Phase 2)
   - Build on IOR foundation
   - Start with IBL-only sampling
   - Test with CompareTransmission scene

3. **Volume** (Phase 3)
   - Requires transmission to be meaningful
   - Test with CompareVolume scene
