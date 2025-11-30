# TRANSPARENCY SHADING IMPLEMENTATION PLAN

This document outlines the plan for implementing proper PBR shading for transparent materials in the fragment shader.

## CONTEXT

The renderer uses two fundamentally different rendering approaches:

### Opaque Materials (Deferred/Visibility Buffer)
- **Pipeline**: Geometry pass → Material compute pass
- **Geometry pass**: Writes triangle IDs, barycentric coords, normals to G-buffer textures
- **Material pass**: Compute shader reads G-buffer, manually interpolates attributes, performs lighting
- **Data access**: All data comes from storage buffers, indexed by triangle ID
- **MSAA**: Manual per-sample shading in compute shader (unrolled loops)
- **Texture sampling**: Uses `textureSampleGrad()` with manually computed gradients

### Transparent Materials (Forward Rendering)
- **Pipeline**: Single geometry+material pass
- **Vertex shader**: Transforms positions, normals, tangents; applies morphs/skinning
- **Fragment shader**: Receives interpolated data, samples textures, performs lighting
- **Data access**: Interpolated from vertex shader + storage buffers for custom attributes
- **MSAA**: Hardware MSAA with resolve targets (no manual per-sample code needed)
- **Texture sampling**: Uses `textureSample()` with automatic derivatives from hardware

## KEY ARCHITECTURAL DIFFERENCES

| Aspect | Opaque (Compute) | Transparent (Fragment) |
|--------|-----------------|----------------------|
| **Position** | Reconstructed from depth + camera rays | Interpolated via `@builtin(position)` |
| **Normal/Tangent** | Read from packed G-buffer texture | Interpolated from vertex shader via `@location` |
| **Triangle Index** | Read from visibility G-buffer | NOT AVAILABLE (forward rendering) |
| **Barycentric** | Read from barycentric G-buffer | NOT AVAILABLE (or use extensions) |
| **Custom Attributes** | Manual interpolation using barycentric + storage buffer | Manual lookup using `gl_PrimitiveID` + storage buffer |
| **MSAA** | Manual per-sample iteration | Hardware resolve |
| **Texture Gradients** | Manually computed from barycentric derivatives | Automatic hardware derivatives |
| **Lighting** | Deferred, per-pixel compute dispatch | Forward, immediate in fragment shader |

## ANSWERS TO SPECIFIC QUESTIONS

### 1. MSAA Handling

**Answer: Use hardware MSAA with resolve targets. NO manual per-sample code needed.**

For transparency with forward rendering:
- Enable MSAA on the render pipeline (already done via `MultisampleState`)
- Configure MSAA-enabled textures with `sample_count > 1`
- Use a resolve target when you want the final resolved image
- The GPU automatically handles per-sample rasterization and blending

**No need for:**
- Manual per-sample loops like opaque compute shader
- Reading from multisampled textures
- Sample-based branching

The opaque material pass needs manual MSAA because compute shaders can't use hardware MSAA. Fragment shaders get this for free.

### 2. Texture Sampling

**Answer: Use `textureSample()` with automatic derivatives. NO manual gradient computation needed.**

Fragment shaders have access to hardware-computed screen-space derivatives:
- `textureSample(texture, sampler, uv)` - GPU automatically computes mipmap level from pixel quad derivatives
- No need for `textureSampleGrad()` or `textureSampleLevel()`
- No need to manually compute UV gradients from barycentric derivatives

**Advantages:**
- Simpler code
- Automatic mipmap selection
- Proper anisotropic filtering support
- Hardware-optimized

**When to use alternatives:**
- `textureSampleLevel()`: If you want explicit LOD control (not needed for standard PBR)
- `textureSampleGrad()`: If you want custom gradient control (not needed for forward rendering)

## DATA FLOW COMPARISON

### Opaque Material Pass (Compute Shader)
```
G-Buffer Textures (visibility_data, barycentric, normal_tangent, depth)
    ↓
Compute shader reads pixel
    ↓
Extract triangle_index from visibility_data
    ↓
Look up mesh_meta from storage buffer using material_meta_offset
    ↓
Look up original triangle indices from attribute_indices storage buffer
    ↓
Manually interpolate UVs using barycentric + attribute_data storage buffer
    ↓
Sample textures using textureSampleGrad with computed gradients
    ↓
Reconstruct world position from depth
    ↓
Apply PBR lighting
    ↓
Write color to output texture
```

### Transparent Material Pass (Fragment Shader)
```
Vertex shader outputs (world_normal, world_tangent, @builtin(position))
    ↓
Fragment shader receives interpolated data
    ↓
NEED: Access to vertex_index or primitive_id to look up UVs
    ↓
Look up mesh_meta from uniform/storage
    ↓
Look up vertex UVs from attribute_data storage buffer
    ↓
Sample textures using textureSample (automatic derivatives)
    ↓
Compute world position from clip_position and depth
    ↓
Apply PBR lighting
    ↓
Return color from fragment shader (blending handled by pipeline)
```

## CHALLENGE: ACCESSING CUSTOM ATTRIBUTES

The main challenge for transparency is accessing custom attributes (UVs, colors) without triangle_index or barycentric coordinates.

### Problem
- Opaque: Has `triangle_index` from G-buffer → looks up triangle's 3 vertex indices → interpolates UVs with barycentric
- Transparent: No triangle_index available in standard WGSL fragment shader

### Solutions (in order of preference)

#### Option 1: Pass UVs as Vertex Attributes (RECOMMENDED)
**Best approach for transparency forward rendering**

Include UVs (and colors) directly in the transparency vertex buffer:
- Vertex format: position (12) + normal (12) + tangent (16) + uv0 (8) + uv1 (8) + ... = variable stride
- Hardware interpolates UVs automatically
- No manual interpolation needed
- Most efficient and simple

**Trade-offs:**
- Increases vertex buffer size (but still smaller than exploded visibility buffer!)
- Need to create separate transparency vertex buffers per unique attribute set
- Standard approach for forward rendering

**Implementation:**
```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv0: vec2<f32>,  // Add UVs to vertex buffer
    @location(4) uv1: vec2<f32>,  // Optional second UV set
    // ... more attributes as needed
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_tangent: vec4<f32>,
    @location(2) uv0: vec2<f32>,  // Pass through
    @location(3) uv1: vec2<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    // UVs automatically interpolated by hardware!
    let base_color = textureSample(texture, sampler, input.uv0);
    // ...
}
```

#### Option 2: Use `@builtin(vertex_index)` in Fragment Shader
**Possible but limited**

WGSL doesn't provide `@builtin(vertex_index)` in fragment shaders. However, you could:
- Pass `vertex_index` from vertex shader as `@location`
- Interpolation will break this (gets interpolated across triangle as floats)
- Only works with `@interpolate(flat)` to disable interpolation
- Only gives you one vertex index per triangle, not all three

**Not recommended** - doesn't solve the interpolation problem.

#### Option 3: Use `@builtin(primitive_index)` (WebGPU Extension)
**Future-proof but not widely available yet**

WebGPU has a proposal for `@builtin(primitive_index)` which would give triangle_index in fragment shader:
- Could then look up triangle indices from storage buffer
- Would need barycentric coordinates to interpolate (see Option 4)
- Not available in stable WebGPU yet

**Not recommended** - wait for wider support.

#### Option 4: Fragment Shader Barycentric Extension
**WebGPU feature but adds complexity**

WebGPU supports barycentric coordinates as an optional feature:
```wgsl
@builtin(barycentric_coords) bary: vec3<f32>
```

Combined with primitive_index, could replicate opaque approach:
- Get primitive_id → look up triangle indices → interpolate UVs with barycentric

**Trade-offs:**
- Requires optional WebGPU feature (not guaranteed available)
- More complex than Option 1
- Recreates compute shader approach in fragment shader (why?)
- Fragment shaders exist to interpolate data - use them!

**Not recommended** - Option 1 is simpler and standard.

### Recommendation: Option 1 (Pass UVs in Vertex Buffer)

**This is the standard approach for forward rendering and should be used for transparency.**

## IMPLEMENTATION PLAN

### Phase 1: Extend Transparency Vertex Buffer with UVs

**File: `crates/renderer/src/gltf/buffers/mesh/transparency.rs`**

Current format (40 bytes):
```rust
// position (12) + normal (12) + tangent (16) = 40 bytes
```

New format (variable stride):
```rust
// position (12) + normal (12) + tangent (16) + uvs (8 * uv_set_count) + colors (12 * color_set_count) = variable
```

**Changes needed:**
1. Modify `create_transparency_vertices()` to include UVs and colors
2. Update `MeshBufferVertexInfo` to track transparency vertex stride (currently assumes constant 40 bytes)
3. Update vertex buffer layout in `transparent/pipeline.rs` to include UV attributes

**Implementation:**
```rust
// In create_transparency_vertices():
for vertex_index in 0..vertex_count {
    // Position (12 bytes)
    write_vec3(position);

    // Normal (12 bytes)
    write_vec3(normal);

    // Tangent (16 bytes)
    write_vec4(tangent);

    // UVs (8 bytes per set)
    for uv_set in 0..uv_set_count {
        write_vec2(uv);
    }

    // Colors (12 bytes per set) - RGB as vec3<f32>
    for color_set in 0..color_set_count {
        write_vec3(color);
    }
}
```

**Vertex info tracking:**
```rust
pub struct MeshBufferVertexInfo {
    pub count: usize,
    pub transparency_vertex_stride: usize, // NEW: variable stride
}

impl MeshBufferVertexInfo {
    pub fn transparency_geometry_size(&self) -> usize {
        self.count * self.transparency_vertex_stride
    }
}
```

### Phase 2: Update Pipeline Vertex Layouts

**File: `crates/renderer/src/render_passes/material/transparent/pipeline.rs`**

Update `vertex_buffer_layouts()` to dynamically build vertex attributes:

```rust
fn vertex_buffer_layouts(mesh: &Mesh, buffer_info: &MeshBufferInfo) -> Vec<VertexBufferLayout> {
    let mut attributes = vec![
        // Position at location 0
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        // Normal at location 1
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
        // Tangent at location 2
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 24,
            shader_location: 2,
        },
    ];

    let mut offset = 40; // after position + normal + tangent
    let mut location = 3; // after position, normal, tangent

    // Add UV sets
    for i in 0..buffer_info.uv_set_count {
        attributes.push(VertexAttribute {
            format: VertexFormat::Float32x2,
            offset,
            shader_location: location,
        });
        offset += 8;
        location += 1;
    }

    // Add color sets
    for i in 0..buffer_info.color_set_count {
        attributes.push(VertexAttribute {
            format: VertexFormat::Float32x3,
            offset,
            shader_location: location,
        });
        offset += 12;
        location += 1;
    }

    let mut layouts = vec![VertexBufferLayout {
        array_stride: offset as u64,
        step_mode: None,
        attributes,
    }];

    // Add instancing layout if needed
    if mesh.instanced {
        // ... existing instancing code ...
    }

    layouts
}
```

### Phase 3: Update Vertex Shader

**File: `material_transparent_wgsl/vertex.wgsl` (template)**

Update to pass through UV and color attributes:

```wgsl
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    {% if uv_set_count > 0 %}
        {% for i in 0..uv_set_count %}
            @location({{ 3 + i }}) uv_{{ i }}: vec2<f32>,
        {% endfor %}
    {% endif %}
    {% if color_set_count > 0 %}
        {% for i in 0..color_set_count %}
            @location({{ 3 + uv_set_count + i }}) color_{{ i }}: vec3<f32>,
        {% endfor %}
    {% endif %}
    {% if instancing_transforms %}
        @location({{ 3 + uv_set_count + color_set_count }}) instance_transform_row_0: vec4<f32>,
        // ... other instance transform rows
    {% endif %}
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,  // NEW: needed for lighting
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    {% if uv_set_count > 0 %}
        {% for i in 0..uv_set_count %}
            @location({{ 3 + i }}) uv_{{ i }}: vec2<f32>,
        {% endfor %}
    {% endif %}
    {% if color_set_count > 0 %}
        {% for i in 0..color_set_count %}
            @location({{ 3 + uv_set_count + i }}) color_{{ i }}: vec3<f32>,
        {% endfor %}
    {% endif %}
}

@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Apply morphs/skinning/transforms (already done)
    let applied = apply_vertex(...);

    out.clip_position = applied.clip_position;
    out.world_normal = applied.world_normal;
    out.world_tangent = applied.world_tangent;

    // NEW: Compute world position for lighting
    // This is interpolated and used in fragment shader
    let model_transform = get_model_transform(...);
    let world_pos = model_transform * vec4<f32>(input.position, 1.0);
    out.world_position = world_pos.xyz;

    // Pass through UVs (hardware interpolates)
    {% if uv_set_count > 0 %}
        {% for i in 0..uv_set_count %}
            out.uv_{{ i }} = input.uv_{{ i }};
        {% endfor %}
    {% endif %}

    // Pass through colors (hardware interpolates)
    {% if color_set_count > 0 %}
        {% for i in 0..color_set_count %}
            out.color_{{ i }} = input.color_{{ i }};
        {% endfor %}
    {% endif %}

    return out;
}
```

### Phase 4: Implement Fragment Shader

**File: `material_transparent_wgsl/fragment.wgsl` (template)**

This is where the main work happens. We need to:
1. Access material data
2. Sample PBR textures using interpolated UVs
3. Apply normal mapping
4. Compute lighting (IBL + punctual)
5. Output color with alpha

```wgsl
struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    {% if uv_set_count > 0 %}
        {% for i in 0..uv_set_count %}
            @location({{ 3 + i }}) uv_{{ i }}: vec2<f32>,
        {% endfor %}
    {% endif %}
    {% if color_set_count > 0 %}
        {% for i in 0..color_set_count %}
            @location({{ 3 + uv_set_count + i }}) color_{{ i }}: vec3<f32>,
        {% endfor %}
    {% endif %}
}

struct FragmentOutput {
    @location(0) oit_color: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // 1. Get material data
    // The mesh_meta uniform tells us which material to use
    let material_offset = mesh_meta.material_offset;
    let pbr_material = pbr_get_material(material_offset);

    // 2. Sample base color texture
    var base_color = pbr_material.base_color_factor;
    {% if uv_set_count > 0 %}
        if (pbr_material.has_base_color_texture) {
            // Determine which UV set this texture uses
            let uv_set = pbr_material.base_color_tex_info.uv_set_index;
            let uv = get_uv(input, uv_set);

            // Sample with automatic derivatives!
            let tex_color = texture_pool_sample(pbr_material.base_color_tex_info, uv);
            base_color *= tex_color;
        }
    {% endif %}

    // 3. Apply vertex colors if present
    {% if color_set_count > 0 %}
        if (pbr_material.has_color_info) {
            let color_set = pbr_material.color_info.set_index;
            let vertex_color = get_vertex_color(input, color_set);
            base_color.rgb *= vertex_color;
        }
    {% endif %}

    // 4. Sample metallic-roughness texture
    var metallic = pbr_material.metallic_factor;
    var roughness = pbr_material.roughness_factor;
    {% if uv_set_count > 0 %}
        if (pbr_material.has_metallic_roughness_texture) {
            let uv_set = pbr_material.metallic_roughness_tex_info.uv_set_index;
            let uv = get_uv(input, uv_set);
            let tex = texture_pool_sample(pbr_material.metallic_roughness_tex_info, uv);
            metallic *= tex.b;  // Blue channel
            roughness *= tex.g; // Green channel
        }
    {% endif %}

    // 5. Apply normal mapping
    var normal = normalize(input.world_normal);
    {% if uv_set_count > 0 %}
        if (pbr_material.has_normal_texture) {
            let uv_set = pbr_material.normal_tex_info.uv_set_index;
            let uv = get_uv(input, uv_set);

            // Sample normal map
            let tex = texture_pool_sample(pbr_material.normal_tex_info, uv);
            let tangent_normal = vec3<f32>(
                (tex.r * 2.0 - 1.0) * pbr_material.normal_scale,
                (tex.g * 2.0 - 1.0) * pbr_material.normal_scale,
                tex.b * 2.0 - 1.0,
            );

            // Build TBN matrix from interpolated tangent
            let T = normalize(input.world_tangent.xyz);
            let N = normal;
            let B = normalize(cross(N, T)) * input.world_tangent.w;
            let tbn = mat3x3<f32>(T, B, N);

            // Transform tangent-space normal to world space
            normal = normalize(tbn * tangent_normal);
        }
    {% endif %}

    // 6. Sample occlusion texture
    var occlusion = 1.0;
    {% if uv_set_count > 0 %}
        if (pbr_material.has_occlusion_texture) {
            let uv_set = pbr_material.occlusion_tex_info.uv_set_index;
            let uv = get_uv(input, uv_set);
            let tex = texture_pool_sample(pbr_material.occlusion_tex_info, uv);
            occlusion = mix(1.0, tex.r, pbr_material.occlusion_strength);
        }
    {% endif %}

    // 7. Sample emissive texture
    var emissive = pbr_material.emissive_factor;
    {% if uv_set_count > 0 %}
        if (pbr_material.has_emissive_texture) {
            let uv_set = pbr_material.emissive_tex_info.uv_set_index;
            let uv = get_uv(input, uv_set);
            let tex = texture_pool_sample(pbr_material.emissive_tex_info, uv);
            emissive *= tex.rgb;
        }
    {% endif %}
    emissive *= pbr_material.emissive_strength;

    // 8. Create material color structure
    let material_color = PbrMaterialColor(
        base_color,
        vec2<f32>(metallic, roughness),
        normal,
        occlusion,
        emissive,
    );

    // 9. Compute lighting
    let surface_to_camera = normalize(camera.position - input.world_position);

    var color = vec3<f32>(0.0);

    // IBL (Image-Based Lighting)
    {% if has_lighting_ibl %}
        color = brdf_ibl(
            material_color,
            normal,
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

    // Punctual lights (directional, point, spot)
    {% if has_lighting_punctual %}
        let lights_info_data = get_lights_info();
        for (var i = 0u; i < lights_info_data.n_lights; i++) {
            let light = get_light(i);
            let light_brdf = light_to_brdf(light, normal, input.world_position);
            color += brdf_direct(material_color, light_brdf, surface_to_camera);
        }
    {% endif %}

    // 10. Output final color with alpha
    out.oit_color = vec4<f32>(color, base_color.a);

    return out;
}

// Helper to get UV based on set index (template generates this)
fn get_uv(input: FragmentInput, set_index: u32) -> vec2<f32> {
    {% if uv_set_count > 0 %}
        switch (set_index) {
            {% for i in 0..uv_set_count %}
                case {{ i }}u: { return input.uv_{{ i }}; }
            {% endfor %}
            default: { return vec2<f32>(0.0); }
        }
    {% else %}
        return vec2<f32>(0.0);
    {% endif %}
}

// Helper to get vertex color based on set index
fn get_vertex_color(input: FragmentInput, set_index: u32) -> vec3<f32> {
    {% if color_set_count > 0 %}
        switch (set_index) {
            {% for i in 0..color_set_count %}
                case {{ i }}u: { return input.color_{{ i }}; }
            {% endfor %}
            default: { return vec3<f32>(1.0); }
        }
    {% else %}
        return vec3<f32>(1.0);
    {% endif %}
}

// Texture sampling with automatic derivatives (simpler than opaque!)
fn texture_pool_sample(info: TextureInfo, uv: vec2<f32>) -> vec4<f32> {
    // Apply texture transform
    let transformed_uv = texture_transform_uvs(uv, info);

    switch info.array_index {
        {% for i in 0..texture_pool_arrays_len %}
            case {{ i }}u: {
                return _texture_pool_sample(info, pool_tex_{{ i }}, transformed_uv);
            }
        {% endfor %}
        default: {
            return vec4<f32>(0.0);
        }
    }
}

fn _texture_pool_sample(
    info: TextureInfo,
    tex: texture_2d_array<f32>,
    uv: vec2<f32>
) -> vec4<f32> {
    switch info.sampler_index {
        {% for i in 0..texture_pool_samplers_len %}
            case {{ i }}u: {
                // textureSample uses automatic derivatives - no grad needed!
                return textureSample(
                    tex,
                    pool_sampler_{{ i }},
                    uv,
                    i32(info.layer_index)
                );
            }
        {% endfor %}
        default: {
            return vec4<f32>(0.0);
        }
    }
}
```

### Phase 5: Update Shader Template System

**File: `transparent/shader/template.rs`**

Add fields to cache key for UV/color set counts:

```rust
pub struct ShaderCacheKeyMaterialTransparent {
    pub attributes: MeshBufferAttributesMaterialKey,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub msaa_sample_count: Option<u32>,
    pub mipmaps: bool,
    pub instancing_transforms: bool,
    // NEW:
    pub uv_set_count: Option<u32>,
    pub color_set_count: Option<u32>,
}
```

Update template structs:

```rust
pub struct ShaderTemplateTransparentMaterialVertex {
    pub instancing_transforms: bool,
    pub uv_set_count: Option<u32>,
    pub color_set_count: Option<u32>,
}

pub struct ShaderTemplateTransparentMaterialFragment {
    pub uv_set_count: Option<u32>,
    pub color_set_count: Option<u32>,
    pub has_lighting_ibl: bool,
    pub has_lighting_punctual: bool,
}
```

### Phase 6: Update Bind Groups

**File: `transparent/bind_group.rs`**

Ensure the fragment shader has access to:
- Camera uniform (for camera position)
- Materials storage buffer
- Lights uniform + storage buffer
- Texture pool (textures + samplers)
- Mesh meta uniform (for material offset)

These should mostly already be set up. Verify:

```rust
// Group 0: Main data
// - @binding(0): camera uniform
// - @binding(1): materials storage buffer
// - @binding(2): (other buffers as needed)

// Group 1: Lights
// - @binding(0): lights_info uniform
// - @binding(1): lights storage buffer

// Group 2: Texture pool
// - @binding(0..N): texture arrays
// - @binding(N..M): samplers

// Group 3: Mesh meta
// - @binding(0): mesh_meta uniform
```

### Phase 7: Shared Helper Includes

Many shader functions can be shared between opaque and transparent:

**Already shared** (in `opaque_and_transparency_wgsl/`):
- `pbr/material.wgsl` - Material struct and `pbr_get_material()`
- `pbr/lighting/brdf.wgsl` - PBR BRDF functions
- `pbr/lighting/lights.wgsl` - Light data structures
- `textures.wgsl` - Texture transform helpers
- `vertex_color.wgsl` - Vertex color utilities

**Need new fragment-specific helpers** (in `material_transparent_wgsl/helpers/`):
- `texture_sampling.wgsl` - Fragment shader texture sampling (textureSample vs textureSampleGrad)
- `material_color.wgsl` - Fragment-specific material color sampling
- `uv_helpers.wgsl` - UV set selection helpers

### Phase 8: Testing Strategy

Test transparency shading incrementally:

1. **Simple unlit**: Output constant color + alpha
   - Verify transparency blending works
   - Verify alpha values are correct

2. **Base color only**: Sample base color texture
   - Verify UV interpolation works
   - Verify automatic derivatives work
   - Verify texture transforms apply correctly

3. **Base color + vertex colors**: Multiply with vertex colors
   - Verify color interpolation works

4. **Full PBR textures**: Add metallic-roughness, normal, occlusion, emissive
   - Verify all texture samples work
   - Verify normal mapping works with interpolated tangents

5. **Unlit lighting**: Emissive only
   - Verify emissive contribution

6. **IBL lighting**: Add image-based lighting
   - Verify reflections work
   - Verify indirect diffuse works

7. **Punctual lighting**: Add directional/point/spot lights
   - Verify direct lighting works
   - Verify multiple lights accumulate correctly

8. **MSAA**: Enable MSAA and verify
   - Edges are smooth
   - No performance issues
   - Resolve works correctly

9. **Alpha modes**: Test all alpha modes
   - AlphaMode::Blend (standard transparency)
   - AlphaMode::Mask (alpha cutoff)

## PERFORMANCE CONSIDERATIONS

### Fragment Shader vs Compute Shader

**Advantages of fragment shader approach:**
- Hardware-interpolated attributes (free!)
- Hardware-computed derivatives (free!)
- Hardware MSAA (free!)
- No manual per-sample loops
- Simpler code

**Trade-offs:**
- Slightly larger vertex buffer (includes UVs)
- Still much smaller than opaque exploded buffer
- Standard forward rendering approach

### Memory Comparison (Cube Example)

**Current transparency (40 bytes/vertex, no UVs):**
- 8 vertices × 40 bytes = 320 bytes
- Index buffer: 36 indices × 4 bytes = 144 bytes
- Total: 464 bytes

**With 2 UV sets (56 bytes/vertex):**
- 8 vertices × 56 bytes = 448 bytes
- Index buffer: 36 indices × 4 bytes = 144 bytes
- Total: 592 bytes

**Opaque visibility buffer (exploded, for comparison):**
- 36 vertices × 52 bytes = 1,872 bytes
- Index buffer: 36 indices × 4 bytes = 144 bytes
- Custom attributes: 8 vertices × 16 bytes = 128 bytes
- Total: 2,144 bytes

**Transparency is still 3.6x smaller than opaque!**

## SUMMARY

### Key Takeaways

1. **MSAA**: Use hardware MSAA with resolve targets, no manual code needed
2. **Texture Sampling**: Use `textureSample()` with automatic derivatives
3. **Custom Attributes**: Include UVs/colors in vertex buffer, let hardware interpolate
4. **Lighting**: Reuse existing PBR BRDF functions, same as opaque
5. **Architecture**: Standard forward rendering, much simpler than deferred compute

### What to Reuse from Opaque

- Material struct (`PbrMaterial`, `pbr_get_material()`)
- BRDF functions (`brdf_ibl()`, `brdf_direct()`, `light_to_brdf()`)
- Light data structures (`LightsInfoPacked`, `LightPacked`, etc.)
- Texture transform utilities
- Vertex color utilities

### What's Different from Opaque

- No G-buffer reads
- No manual barycentric interpolation
- No manual gradient computation
- Hardware interpolates UVs, normals, tangents
- Hardware computes derivatives
- Simpler code!

### Implementation Order

1. ✅ Vertex buffer format (add UVs/colors)
2. ✅ Pipeline vertex layouts (dynamic attribute generation)
3. ✅ Vertex shader (pass through UVs/colors)
4. ✅ Fragment shader (sample textures, compute lighting)
5. ⬜ Shader templates (add UV/color count parameters)
6. ⬜ Bind groups (verify all resources available)
7. ⬜ Testing (incremental, test each feature)

This approach follows standard forward rendering practices and leverages GPU hardware features instead of fighting against them. The result is simpler, more maintainable code that performs well.
