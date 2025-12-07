# Transparency Shading Implementation TODO

## Architecture Overview

### Material Opaque (Compute Shader)
- Uses deferred compute approach with visibility/geometry buffers
- Has access to:
  - Barycentric coordinates from `barycentric_tex`
  - Triangle indices from `visibility_data_tex`
  - Raw attribute buffers (`attribute_indices`, `attribute_data`)
  - Object-space vertices via `get_object_space_vertices()`
- Samples all PBR textures using `pbr_get_material_color_grad()` or `pbr_get_material_color_no_mips()`
- These functions perform barycentric interpolation of UVs, normals, tangents, etc.
- Applies lighting via `apply_lighting()` → `brdf_direct()` + `brdf_ibl()`

### Material Transparent (Fragment Shader)
- Traditional rasterization pipeline (vertex → fragment)
- Has access to:
  - Interpolated vertex attributes: `world_normal`, `world_tangent`, `uv_*`, `color_*`
  - No barycentric coordinates
  - No triangle indices
  - No raw attribute buffers
  - No object-space vertex positions
- Currently only samples base color texture
- **Needs lighting and full PBR material sampling**

## The Core Problem

The functions in `material_color.wgsl` are designed for compute shader workflows and **cannot be directly used** in the transparent fragment shader because they require:

1. **Barycentric coordinates** - for interpolating vertex attributes
2. **Triangle indices** - for accessing per-triangle data
3. **Raw attribute buffers** - for reading vertex data directly
4. **Object-space vertices** - for normal mapping fallback (when tangents are missing or degenerate)
5. **Manual UV gradient computation** - for anisotropic filtering in compute shaders

Fragment shaders already have:
- Interpolated UVs (no need for barycentric interpolation)
- Interpolated normals and tangents
- Automatic mip level selection (no need for manual gradients)

## Recommended Solution

### 1. Create `material_color_fragment.wgsl`

Create simplified fragment-shader-friendly versions of the PBR material sampling functions. These should be located in `crates/renderer/src/render_passes/shared/shader/opaque_and_transparency_wgsl/pbr/material_color_fragment.wgsl`.

Key differences from compute version:
- Take interpolated UVs directly (no barycentric math)
- Use `texture_pool_sample()` instead of `texture_pool_sample_grad()` or `texture_pool_sample_no_mips()`
- Build TBN matrix from interpolated `world_normal` and `world_tangent`
- No fallback to UV-based tangent computation (rely on vertex data)
- No vertex color support via attribute buffers (handle separately if needed)

#### Base Color
```wgsl
fn pbr_material_base_color_frag(
    material: PbrMaterial,
    uv: vec2<f32>
) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
        color *= texture_pool_sample(material.base_color_tex_info, uv);
    }
    return color;
}
```

#### Metallic Roughness
```wgsl
fn pbr_material_metallic_roughness_frag(
    material: PbrMaterial,
    uv: vec2<f32>
) -> vec2<f32> {
    var color = vec2<f32>(material.metallic_factor, material.roughness_factor);
    if material.has_metallic_roughness_texture {
        let tex = texture_pool_sample(material.metallic_roughness_tex_info, uv);
        // glTF uses B channel for metallic, G channel for roughness
        color *= vec2<f32>(tex.b, tex.g);
    }
    return color;
}
```

#### Normal Mapping
```wgsl
// Simplified normal mapping using interpolated tangent/normal from vertex shader
// Much simpler than compute version - no fallback paths needed
fn pbr_normal_frag(
    material: PbrMaterial,
    uv: vec2<f32>,
    world_normal: vec3<f32>,
    world_tangent: vec4<f32>  // w = handedness
) -> vec3<f32> {
    if !material.has_normal_texture {
        return normalize(world_normal);
    }

    // Sample normal map
    let tex = texture_pool_sample(material.normal_tex_info, uv);
    let tangent_normal = vec3<f32>(
        (tex.r * 2.0 - 1.0) * material.normal_scale,
        (tex.g * 2.0 - 1.0) * material.normal_scale,
        tex.b * 2.0 - 1.0,
    );

    // Build TBN matrix from interpolated vertex data
    let N = normalize(world_normal);
    let T = normalize(world_tangent.xyz);
    let B = cross(N, T) * world_tangent.w;
    let tbn = mat3x3<f32>(T, B, N);

    return normalize(tbn * tangent_normal);
}
```

#### Occlusion
```wgsl
fn pbr_occlusion_frag(
    material: PbrMaterial,
    uv: vec2<f32>
) -> f32 {
    var occlusion = 1.0;
    if material.has_occlusion_texture {
        let tex = texture_pool_sample(material.occlusion_tex_info, uv);
        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occlusion;
}
```

#### Emissive
```wgsl
fn pbr_emissive_frag(
    material: PbrMaterial,
    uv: vec2<f32>
) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.has_emissive_texture {
        color *= texture_pool_sample(material.emissive_tex_info, uv).rgb;
    }
    color *= material.emissive_strength;
    return color;
}
```

#### Main Material Color Function
```wgsl
fn pbr_get_material_color_frag(
    material: PbrMaterial,
    uv: vec2<f32>,
    world_normal: vec3<f32>,
    world_tangent: vec4<f32>
) -> PbrMaterialColor {
    let base = pbr_material_base_color_frag(material, uv);
    let metallic_roughness = pbr_material_metallic_roughness_frag(material, uv);
    let normal = pbr_normal_frag(material, uv, world_normal, world_tangent);
    let occlusion = pbr_occlusion_frag(material, uv);
    let emissive = pbr_emissive_frag(material, uv);

    return PbrMaterialColor(
        base,
        metallic_roughness,
        normal,
        occlusion,
        emissive
    );
}
```

### 2. Update Fragment Shader to Include World Position

The lighting functions need `world_position` to calculate light direction and attenuation. Two options:

#### Option A: Pass from Vertex Shader (Recommended - more accurate)
```wgsl
// In vertex.wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_tangent: vec4<f32>,
    @location(2) world_position: vec3<f32>,  // ADD THIS
    // ... uvs, colors, etc.
}

@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    // ... existing code ...
    out.world_position = applied.world_position;  // Make sure apply_vertex returns this
    // ...
}
```

#### Option B: Reconstruct in Fragment (Saves interpolator)
```wgsl
// In fragment.wgsl
fn reconstruct_world_position(clip_pos: vec4<f32>, camera: CameraUniform) -> vec3<f32> {
    let ndc = clip_pos.xyz / clip_pos.w;
    let world_pos = camera.inv_view_proj * vec4<f32>(ndc, 1.0);
    return world_pos.xyz / world_pos.w;
}
```

### 3. Update Bind Groups for Transparent Pass

The transparent pass needs access to lighting data that's currently missing. Update `material_transparent_wgsl/bind_groups.wgsl`:

```wgsl
@group(1) @binding(0) var<uniform> lights_info: LightsInfoPacked;
@group(1) @binding(1) var<storage, read> lights: array<LightPacked>;

// Add these for IBL if not already present:
@group(1) @binding(2) var ibl_filtered_env_tex: texture_cube<f32>;
@group(1) @binding(3) var ibl_filtered_env_sampler: sampler;
@group(1) @binding(4) var ibl_irradiance_tex: texture_cube<f32>;
@group(1) @binding(5) var ibl_irradiance_sampler: sampler;
@group(1) @binding(6) var brdf_lut_tex: texture_2d<f32>;
@group(1) @binding(7) var brdf_lut_sampler: sampler;
```

Note: The actual binding indices may need adjustment based on your pipeline layout.

### 4. Update Fragment Shader to Apply Lighting

Update `material_transparent_wgsl/fragment.wgsl`:

```wgsl
@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // Get material
    let material = pbr_get_material(material_mesh_meta.material_offset);

    // Sample all PBR textures using fragment-friendly functions
    let material_color = pbr_get_material_color_frag(
        material,
        input.uv_0,
        input.world_normal,
        input.world_tangent
    );

    // Get world position (choose option A or B from above)
    // Option A: let world_position = input.world_position;
    // Option B:
    let world_position = reconstruct_world_position(input.clip_position, camera);

    // Calculate surface to camera vector
    let surface_to_camera = normalize(camera.position - world_position);

    // Apply lighting (reuse from opaque!)
    let lights_info = get_lights_info();
    let lit_color = apply_lighting(
        material_color,
        surface_to_camera,
        world_position,
        lights_info
    );

    // Output with alpha
    out.color = vec4<f32>(lit_color, material_color.base.a);

    // Alternative: Premultiplied alpha
    // out.color = vec4<f32>(lit_color * material_color.base.a, material_color.base.a);

    return out;
}
```

### 5. Update Includes

Update `material_transparent_wgsl/includes.wgsl` to include the new fragment material functions and lighting:

```wgsl
/*************** START material_color_fragment.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/material_color_fragment.wgsl" %}
/*************** END material_color_fragment.wgsl ******************/

/*************** START brdf.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/lighting/brdf.wgsl" %}
/*************** END brdf.wgsl ******************/

// Note: You may also need to create a simplified apply_lighting for fragments
// or conditionally compile the existing material_shading.wgsl without compute-specific code
```

### 6. Create `material_shading_fragment.wgsl` (Optional)

If the existing `material_shading.wgsl` (which wraps `compute_material_color` and `apply_lighting`) is too compute-specific, create a fragment version:

`crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/helpers/material_shading_fragment.wgsl`

```wgsl
// Apply all enabled lighting to a material and return the final color
fn apply_lighting_frag(
    material_color: PbrMaterialColor,
    surface_to_camera: vec3<f32>,
    world_position: vec3<f32>,
    lights_info: LightsInfo,
) -> vec3<f32> {
    var color = vec3<f32>(0.0);

    {% if has_lighting_ibl() %}
        color = brdf_ibl(
            material_color,
            material_color.normal,
            surface_to_camera,
            ibl_filtered_env_tex,
            ibl_filtered_env_sampler,
            ibl_irradiance_tex,
            ibl_irradiance_sampler,
            brdf_lut_tex,
            brdf_lut_sampler,
            lights_info.ibl
        );
    {% endif %}

    {% if has_lighting_punctual() %}
        for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
            let light_brdf = light_to_brdf(get_light(i), material_color.normal, world_position);
            color += brdf_direct(material_color, light_brdf, surface_to_camera);
        }
    {% endif %}

    return color;
}
```

## Additional Considerations

### Vertex Colors
If transparent materials need vertex colors:
- **Option 1**: Pass through vertex shader (add to VertexOutput)
- **Option 2**: Multiply in fragment after material sampling
- **Option 3**: Skip vertex colors for transparent objects

Example if supporting:
```wgsl
// After getting base color
if material.has_color_info {
    base *= input.color_0;  // Assuming color_0 is available in FragmentInput
}
```

### Alpha Handling
Transparent materials require careful alpha handling:

1. **Alpha Cutoff (Mask Mode)**
   ```wgsl
   if material.alpha_mode == ALPHA_MODE_MASK {
       if material_color.base.a < material.alpha_cutoff {
           discard;
       }
   }
   ```

2. **Blend Mode**
   - Output non-premultiplied alpha
   - Configure blend state on CPU side
   - Or premultiply in shader: `vec4(color * alpha, alpha)`

3. **Alpha to Coverage** (MSAA)
   - May need `@builtin(sample_mask) sample_mask: u32` output
   - Helps with alpha-tested foliage on transparency pass

### Double-Sided Materials
Check `material.double_sided` and flip normal if needed:
```wgsl
var world_normal = input.world_normal;
if material.double_sided && !is_front_face {
    world_normal = -world_normal;
}
```

Add `@builtin(front_facing) is_front_face: bool` to FragmentInput if needed.

### Mipmaps
Fragment shaders automatically compute mip levels via screen-space derivatives. No manual gradient computation needed (unlike compute shaders).

### Sorting
Transparent objects typically need back-to-front sorting on CPU side for correct blending. Consider:
- Sort by distance from camera
- Or use depth peeling / order-independent transparency
- Current approach (single transparency pass) works for simple cases

### Performance
- Fragment shading is more expensive than compute for opaque (deferred is faster)
- But necessary for transparency due to blending requirements
- Consider batching transparent draws by material to reduce state changes

## Implementation Checklist

- [ ] Create `material_color_fragment.wgsl` with all fragment-friendly PBR sampling functions
- [ ] Update vertex shader to pass world position (or add reconstruction function)
- [ ] Update fragment shader input struct to include world position (if passing from vertex)
- [ ] Update transparent bind groups to include lights and IBL resources
- [ ] Update fragment shader to call `pbr_get_material_color_frag()` and `apply_lighting()`
- [ ] Update `includes.wgsl` to include new files (material_color_fragment, brdf, lights)
- [ ] Handle vertex colors if needed (pass through vertex shader or skip)
- [ ] Implement alpha mode handling (mask, blend, premultiply)
- [ ] Test with various materials (textured, untextured, normal mapped)
- [ ] Test with punctual lights (directional, point, spot)
- [ ] Test with IBL (if enabled)
- [ ] Verify normal mapping works correctly
- [ ] Check double-sided materials
- [ ] Update Rust side to bind lighting resources to transparent pass

## Files to Modify

1. **New file**: `crates/renderer/src/render_passes/shared/shader/opaque_and_transparency_wgsl/pbr/material_color_fragment.wgsl`
2. **Modify**: `crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/vertex.wgsl`
3. **Modify**: `crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/fragment.wgsl`
4. **Modify**: `crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/bind_groups.wgsl`
5. **Modify**: `crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/includes.wgsl`
6. **Modify**: Rust code for transparent pass pipeline creation and bind group setup

## References

- Opaque compute shader: `material_opaque_wgsl/compute.wgsl`
- Material color functions: `opaque_and_transparency_wgsl/pbr/material_color.wgsl`
- Lighting functions: `opaque_and_transparency_wgsl/pbr/lighting/brdf.wgsl`
- Material shading helper: `material_opaque_wgsl/helpers/material_shading.wgsl`
