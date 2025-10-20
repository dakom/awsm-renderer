- SKYBOX (get this working first so we know cubemaps are correct)

# Refactor bind groups - megatexture can be its own bind group in opaque pipeline

# TODO: Lighting Implementation

## Current Status

The flicker bug in InterpolationTest has been fixed! The issue was NOT with `world_position` calculation (which is correct), but with the lighting model structure.

### What Was Fixed

1. **Removed discontinuity**: Changed from `if (n_dot_l > 0.0001) { brdf() } else { unlit() }` to always calling `brdf()` without the if statement
2. **Multiple lights**: Added 4 directional lights from different angles for smooth transitions during rotation
3. **IBL compensation**: Reduced IBL intensity constants in `brdf.wgsl` since IBL gets added 4 times (once per light)

### Current Workarounds & Issues

The current implementation has several hacks that need proper fixes:

1. **IBL is added multiple times**: The `brdf()` function includes IBL contribution, and we call it once per light. This means IBL gets added 4x instead of 1x. We compensated by reducing `ENV_INTENSITY_DIFF` and `ENV_INTENSITY_SPEC` by 4x.

2. **No proper ambient occlusion**: The `unlit()` function is no longer being called, so AO from materials isn't being applied correctly.

3. **High light intensities**: To compensate for the reduced IBL, direct light intensities are higher than they should be (3.5, 2.2, 1.5, 1.2).

## Next Steps to Proper Lighting

### 1. Separate Direct Lighting from IBL in BRDF

**Current**: `brdf()` returns `Lo + Fd_indir + Fs_indir + emissive` (line 111 in `brdf.wgsl`)

**Goal**: Split into two functions:
- `brdf_direct()`: Returns only `Lo` (direct lighting from one light)
- `brdf_ibl()`: Returns `Fd_indir + Fs_indir + emissive` (indirect + emissive)

**Implementation**:
```wgsl
// In compute.wgsl:
var color = vec3<f32>(0.0);

// Add direct lighting from each light
for(var i = 0u; i < n_lights; i = i + 1u) {
    let light_brdf = light_to_brdf(get_light(i), world_normal, standard_coordinates.world_position);
    color += brdf_direct(material_color, light_brdf, standard_coordinates.surface_to_camera);
}

// Add IBL once
color += brdf_ibl(material_color, world_normal, standard_coordinates.surface_to_camera);
```

### 2. Implement Real IBL

**Current**: Stub functions with hardcoded colors and fake hemisphere sampling

**Goal**: Sample from actual environment maps

**Tasks**:
- [ ] Load HDR environment map (equirectangular or cubemap)
- [ ] Pre-compute irradiance map for diffuse IBL
- [ ] Pre-compute prefiltered environment map for specular IBL
- [ ] Pre-compute or use analytic BRDF integration LUT
- [ ] Update `sampleIrradianceStub()`, `samplePrefilteredEnvStub()`, `sampleBRDFLUTStub()` to sample from real textures

**Files to modify**:
- `brdf.wgsl`: Replace stub functions with real texture sampling
- Add IBL texture bindings to material pipeline

### 3. Restore `unlit()` for Unlit Materials

**Current**: `unlit()` function exists but isn't being called

**Goal**: Use `unlit()` only for materials that are actually unlit (no lighting calculation)

**Implementation**:
- Add a material flag for "unlit" materials
- In compute shader, check this flag and branch:
  - If unlit: `color = unlit(material_color)`
  - If lit: Use the direct + IBL approach from step 1

### 4. Proper Scene Lighting Configuration

Once the above is done, you can set more realistic light intensities and rely on IBL for ambient/fill lighting:

**Suggested starting point**:
- 1-2 directional lights with intensities around 1.0-2.0
- Proper IBL providing ambient/indirect lighting
- Let materials' roughness/metallic properties control the look

### 5. Optional: Light Management System

Consider adding a proper light management system:
- [ ] Support for more light types (currently only directional and point)
- [ ] Dynamic light arrays (not hardcoded switch statement)
- [ ] Light culling for scenes with many lights
- [ ] Shadow mapping

## Testing

After each step, verify:
- ✅ InterpolationTest: Smooth rotation with no flicker
- ✅ BoxTextured: Proper material colors, not washed out
- ✅ Various roughness/metallic combinations look correct
- ✅ Specular highlights appear natural

## Summary

The core rendering (world position reconstruction, normals, etc.) is working correctly. The main issue is the lighting model architecture - specifically how direct lighting and IBL are combined. The priority fix is step 1 (separate direct from IBL in BRDF), which will eliminate the workarounds and allow for proper lighting.
