# Transparent Pass Vertex Attributes Implementation Plan

## Executive Summary

This document outlines the plan to implement vertex attribute buffers for the transparent material render pass. Currently, the opaque pass uses storage buffers with manual indexing in compute shaders, while the transparent pass uses a traditional vertex/fragment shader pipeline. The goal is to enable the transparent pass to access custom vertex attributes (UVs, colors, joints, weights) via vertex buffer bindings rather than storage buffers.

## Current Architecture

### Opaque Material Pass (Storage Buffer Approach)
The opaque material pass uses a compute shader that manually loads vertex attributes from storage buffers:

```wgsl
// In compute.wgsl
@group(0) @binding(X) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(Y) var<storage, read> attribute_data: array<f32>;

// Manual indexing to load UV data
fn _texture_uv_per_vertex(...) -> vec2<f32> {
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    let uv_offset = uv_sets_index + (set_index * 2u);
    let index = vertex_start + uv_offset;
    let uv = vec2<f32>(attribute_data[index], attribute_data[index + 1]);
    return uv;
}
```

**Key metadata from MeshMeta:**
- `vertex_attribute_stride` - stride across all attributes in floats (NOT bytes)
- `vertex_attribute_data_offset` - offset into attribute_data buffer in floats
- `vertex_attribute_indices_offset` - offset into attribute_indices buffer
- `uv_sets_index` - offset to TEXCOORD_0 within a vertex's data
- `uv_set_count` - number of UV sets
- `color_set_count` - number of color sets

### Transparent Pass (Current State)
The transparent pass currently only receives geometry data (position, normal, tangent, barycentric) via vertex buffers:

**Vertex Buffer Bindings:**
- Slot 0: Visibility data (positions, triangle_index, barycentric, normals, tangents)
  - Stride: 52 bytes
  - Locations 0-4
- Slot 1: Instance transforms (if instanced)
  - Stride: 64 bytes
  - Locations 5-8

**Placeholder in `vertex.rs:11-32`:**
```rust
pub fn vertex_buffer_layout(mesh: &Mesh) -> VertexBufferLayout {
    let start_location = match mesh.instanced {
        true => { /* locations 0-8 used */ }
        false => { /* locations 0-4 used */ }
    } as u32;

    VertexBufferLayout {
        array_stride: 0,  // TODO - calculate dynamically
        step_mode: None,
        attributes: vec![],  // TODO - populate based on mesh attributes
    }
}
```

## Data Flow Analysis

### Attribute Data Buffer Structure

The attribute data is stored in an **interleaved format** per vertex in `meshes.attribute_data_buffers`:

```
Vertex 0: [COLOR_0 data][COLOR_1 data]...[TEXCOORD_0 data][TEXCOORD_1 data]...[JOINTS_0][WEIGHTS_0]...
Vertex 1: [COLOR_0 data][COLOR_1 data]...[TEXCOORD_0 data][TEXCOORD_1 data]...[JOINTS_0][WEIGHTS_0]...
...
```

**Attribute Types & Sizes:**
- `Colors`: 4 floats per set (vec4<f32>), 16 bytes
- `TexCoords`: 2 floats per set (vec2<f32>), 8 bytes
- `Joints`: 4 u32s per set (vec4<u32>), 16 bytes (but stored as floats, bitcast later)
- `Weights`: 4 floats per set (vec4<f32>), 16 bytes

**Important:** The stride is given in **floats** in mesh_meta, but WebGPU vertex buffers expect **bytes**.

### Buffer Locations (Rust Side)

**Per-mesh data access (see `mesh/meshes.rs:255-262`):**
```rust
// GPU buffer containing all attribute data
pub fn attribute_data_gpu_buffer(&self) -> &web_sys::GpuBuffer

// Byte offset for a specific mesh
pub fn attribute_data_buffer_offset(&self, key: MeshKey) -> Result<usize>
```

**Buffer info (see `mesh/buffer_info.rs:64-86`):**
```rust
pub struct MeshBufferTriangleInfo {
    pub vertex_attributes: Vec<MeshBufferVertexAttributeInfo>,
    pub vertex_attributes_size: usize,
    // ...

    pub fn vertex_attribute_stride(&self) -> usize {
        self.vertex_attributes.iter()
            .map(|attr| attr.vertex_size())
            .sum()
    }
}
```

### Current Mesh Render Commands (mesh.rs:147-194)

```rust
pub fn push_material_transparent_pass_commands(...) {
    // Set bind group with dynamic offset for mesh_meta
    render_pass.set_bind_group(3, mesh_meta_bind_group, Some(&[meta_offset]))?;

    // Slot 0: Visibility data (positions, normals, tangents, etc.)
    render_pass.set_vertex_buffer(
        0,
        ctx.meshes.visibility_data_gpu_buffer(),
        Some(ctx.meshes.visibility_data_buffer_offset(mesh_key)? as u64),
        None,
    );

    // Slot 1: Instance transforms (if instanced)
    if let Ok(offset) = ctx.instances.transform_buffer_offset(self.transform_key) {
        render_pass.set_vertex_buffer(1, ...);
    }

    // TODO: Slot 2 (or 1 if not instanced): Attribute data buffer

    // Set index buffer and draw
    render_pass.set_index_buffer(...);
    render_pass.draw_indexed(...);
}
```

## Implementation Plan

### Phase 1: Rust-side Vertex Buffer Layout

**Location:** `crates/renderer/src/render_passes/material/transparent/shader/vertex.rs:11-33`

**Implementation Details:**

1. **Calculate the starting shader location:**
   - Non-instanced: Start at location 5 (after 0-4 used by visibility data)
   - Instanced: Start at location 9 (after 0-8 used by visibility + instance data)

2. **Calculate array_stride:**
   ```rust
   let buffer_info = mesh_buffer_infos.get(mesh.buffer_info_key)?;
   let array_stride = buffer_info.triangles.vertex_attribute_stride() as u64;
   ```

3. **Build vertex attributes dynamically:**
   - Iterate through `buffer_info.triangles.vertex_attributes`
   - For each attribute, create a `VertexAttribute` with:
     - `format`: Determine from attribute type (Float32x2, Float32x4, etc.)
     - `offset`: Cumulative offset in bytes
     - `shader_location`: Sequential starting from `start_location`

**Example attribute mapping:**
```rust
let mut attributes = vec![];
let mut current_offset = 0;
let mut current_location = start_location;

for attr in &buffer_info.triangles.vertex_attributes {
    match attr {
        MeshBufferVertexAttributeInfo::Custom(custom) => match custom {
            MeshBufferCustomVertexAttributeInfo::Colors { .. } => {
                attributes.push(VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: current_offset,
                    shader_location: current_location,
                });
                current_location += 1;
                current_offset += 16; // vec4<f32>
            }
            MeshBufferCustomVertexAttributeInfo::TexCoords { .. } => {
                attributes.push(VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: current_offset,
                    shader_location: current_location,
                });
                current_location += 1;
                current_offset += 8; // vec2<f32>
            }
            // Similar for Joints and Weights
            _ => {}
        },
        _ => {}
    }
}
```

**Important Considerations:**
- Handle multiple UV sets (TEXCOORD_0, TEXCOORD_1, etc.) - each gets its own location
- Handle multiple color sets (COLOR_0, COLOR_1, etc.) - each gets its own location
- The order must match the interleaved buffer layout

### Phase 2: Update Mesh Render Commands

**Location:** `crates/renderer/src/mesh.rs:147-194` in `push_material_transparent_pass_commands()`

**Add after setting slot 0 and optional slot 1:**

```rust
// Slot N: Attribute data buffer (where N = 2 if instanced, 1 if not)
let attribute_buffer_slot = if self.instanced { 2 } else { 1 };

render_pass.set_vertex_buffer(
    attribute_buffer_slot,
    ctx.meshes.attribute_data_gpu_buffer(),
    Some(ctx.meshes.attribute_data_buffer_offset(mesh_key)? as u64),
    None,
);
```

### Phase 3: WGSL Shader Updates

#### 3.1 Update VertexInput Struct
**Location:** `crates/renderer/src/render_passes/shared/shader/geometry_and_transparency_wgsl/vertex/apply.wgsl`

This is the challenging part because the shader needs to be **dynamically generated** based on which attributes are present.

**Template-based approach (using the existing template system):**

```wgsl
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) triangle_index: u32,
    @location(2) barycentric: vec2<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent: vec4<f32>,
    {% if instancing_transforms %}
    @location(5) instance_transform_row_0: vec4<f32>,
    @location(6) instance_transform_row_1: vec4<f32>,
    @location(7) instance_transform_row_2: vec4<f32>,
    @location(8) instance_transform_row_3: vec4<f32>,
    {% endif %}

    // Dynamic attributes start here
    {% match color_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                @location({{ get_next_location() }}) color_{{ i }}: vec4<f32>,
            {% endfor %}
        {% when None %}
    {% endmatch %}

    {% match uv_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                @location({{ get_next_location() }}) uv_{{ i }}: vec2<f32>,
            {% endfor %}
        {% when None %}
    {% endmatch %}

    // Similar for joints and weights
};
```

**Alternative: Pass attributes through to fragment shader**

Instead of complex templating, pass the raw attribute data through:

```wgsl
struct VertexOutput {
    @builtin(position) screen_position: vec4<f32>,
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_index: u32,
    @location(3) barycentric: vec2<f32>,
    @location(4) world_normal: vec3<f32>,
    @location(5) world_tangent: vec4<f32>,

    // Attribute data - templated based on shader variant
    {% match color_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                @location({{ 6 + loop.index0 }}) color_{{ i }}: vec4<f32>,
            {% endfor %}
        {% when None %}
    {% endmatch %}

    {% match uv_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                @location({{ 6 + color_count + loop.index0 }}) uv_{{ i }}: vec2<f32>,
            {% endfor %}
        {% when None %}
    {% endmatch %}
};

fn apply_vertex(vertex_orig: VertexInput) -> VertexOutput {
    // ... existing transform logic ...

    // Pass through attribute data
    {% match color_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                out.color_{{ i }} = vertex_orig.color_{{ i }};
            {% endfor %}
        {% when None %}
    {% endmatch %}

    {% match uv_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                out.uv_{{ i }} = vertex_orig.uv_{{ i }};
            {% endfor %}
        {% when None %}
    {% endmatch %}

    return out;
}
```

#### 3.2 Update Fragment Shader
**Location:** `crates/renderer/src/render_passes/material/transparent/shader/material_transparent_wgsl/fragment.wgsl`

```wgsl
struct FragmentInput {
    @builtin(position) screen_position: vec4<f32>,
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_index: u32,
    @location(3) barycentric: vec2<f32>,
    @location(4) world_normal: vec3<f32>,
    @location(5) world_tangent: vec4<f32>,

    // Add attribute inputs
    {% match color_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                @location({{ 6 + loop.index0 }}) color_{{ i }}: vec4<f32>,
            {% endfor %}
        {% when None %}
    {% endmatch %}

    {% match uv_sets %}
        {% when Some with (count) %}
            {% for i in 0..count %}
                @location({{ 6 + color_count + loop.index0 }}) uv_{{ i }}: vec2<f32>,
            {% endfor %}
        {% when None %}
    {% endmatch %}
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // Now you can use input.uv_0, input.color_0, etc.
    // Access material data via mesh_meta uniform
    let material = pbr_get_material(mesh_meta.material_offset);

    // Sample textures using the vertex UVs
    {% match uv_sets %}
        {% when Some with (_) %}
            let base_color = texture_pool_sample(
                material.base_color_tex_info,
                input.uv_0  // Use the interpolated UV from vertex shader
            );
        {% when None %}
            let base_color = material.base_color_factor;
    {% endmatch %}

    // Apply vertex colors if present
    {% match color_sets %}
        {% when Some with (_) %}
            base_color *= input.color_0;
        {% when None %}
    {% endmatch %}

    out.oit_color = base_color;
    return out;
}
```

### Phase 4: Shader Cache Key Updates

**Location:** `crates/renderer/src/render_passes/material/transparent/shader/cache_key.rs`

The `ShaderCacheKeyMaterialTransparent` already has an `attributes` field:

```rust
pub struct ShaderCacheKeyMaterialTransparent {
    pub attributes: MeshBufferInfoAttributes,  // Already exists!
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub msaa_sample_count: Option<u32>,
    pub mipmaps: bool,
    pub instancing_transforms: bool,
}
```

This means shader variants are already being generated per unique attribute combination. The template system should be extended to use these attributes to generate the correct locations.

### Phase 5: Remove Storage Buffer Bindings (Optional)

Once attributes are working via vertex buffers, the transparent pass no longer needs:
- `@group(X) @binding(Y) var<storage, read> attribute_indices`
- `@group(X) @binding(Y) var<storage, read> attribute_data`

These can remain in the bind group layout for now but should not be used in the shader.

## Key Technical Challenges

### Challenge 1: Location Calculation Complexity

**Problem:** Calculating the correct shader location for each attribute is non-trivial because:
- The number of attributes varies per mesh
- Each attribute type has a different size
- Instancing affects the starting location

**Solution:** Create a helper function that walks through the attribute list and calculates locations:

```rust
fn calculate_attribute_locations(
    mesh: &Mesh,
    buffer_info: &MeshBufferInfo,
) -> Vec<(MeshBufferVertexAttributeInfo, u32, u32)> {
    // Returns: (attribute_type, shader_location, byte_offset)
    let start_location = if mesh.instanced { 9 } else { 5 };
    let mut locations = vec![];
    let mut current_location = start_location;
    let mut current_offset = 0;

    for attr in &buffer_info.triangles.vertex_attributes {
        locations.push((attr.clone(), current_location, current_offset));
        current_location += 1;
        current_offset += attr.vertex_size() as u32;
    }

    locations
}
```

### Challenge 2: Template System Integration

**Problem:** The existing template system needs to know about attribute locations to generate correct WGSL.

**Solution:** Extend the template context with location information:

```rust
// In shader template generation
context.insert("attribute_locations", &calculate_attribute_locations(mesh, buffer_info));
context.insert("next_location_base", if mesh.instanced { 9 } else { 5 });
```

### Challenge 3: Dynamic Offset vs Fixed Offset

**Problem:** Currently using dynamic offsets for mesh_meta bind group. Should we also use dynamic offsets for attribute buffer?

**Current approach:**
```rust
render_pass.set_vertex_buffer(
    slot,
    buffer,
    Some(offset),  // Static offset per draw call
    None,
);
```

**Analysis:**
- Vertex buffer offsets in WebGPU are set per `set_vertex_buffer` call
- This is different from bind group dynamic offsets
- Current approach is correct: use static offset per draw call

### Challenge 4: Attribute Order Consistency

**Problem:** Must ensure Rust-side vertex buffer layout matches the actual buffer data order.

**Solution:** Both must follow the order in `buffer_info.triangles.vertex_attributes`, which is determined during GLTF load:
1. COLOR_0, COLOR_1, ... (in order)
2. TEXCOORD_0, TEXCOORD_1, ... (in order)
3. JOINTS_0, JOINTS_1, ...
4. WEIGHTS_0, WEIGHTS_1, ...

See `mesh/meta/material_opaque_meta.rs:38-61` for the `calculate_uv_sets_index()` function which shows this ordering.

## Verification Strategy

### Step 1: Verify Buffer Layout
Add debug logging to print:
- Array stride from Rust
- Attribute offsets from Rust
- First few bytes of attribute_data buffer

### Step 2: Verify Shader Generation
- Ensure templating generates correct locations
- Check that location numbering is sequential with no gaps
- Verify all locations are used in both vertex and fragment shaders

### Step 3: Render Tests
1. Start with a simple case: single UV set, no colors
2. Render a textured transparent object
3. Verify UVs are correct by examining texture sampling
4. Add vertex colors and verify blending
5. Test with multiple UV sets

### Step 4: Compare with Opaque Pass
- The same mesh should render identically in opaque vs transparent (minus alpha)
- Use a side-by-side comparison with known test assets

## Migration Path

### Minimal Implementation (Phase 1)
1. Implement `vertex_buffer_layout()` for a single UV set only
2. Update `push_material_transparent_pass_commands()` to set attribute buffer
3. Hard-code vertex shader to expect one UV at location 5 (or 9 if instanced)
4. Hard-code fragment shader to use `@location(5)` (or 9)
5. Test with simple textured transparent object

### Full Implementation (Phase 2)
1. Extend template system to handle arbitrary attribute counts
2. Generate shader variants per attribute combination (already partially done via cache key)
3. Test with complex meshes having multiple UVs and colors

### Cleanup (Phase 3)
1. Remove unused storage buffer bindings from bind group layouts
2. Update documentation
3. Performance profiling: vertex buffers vs storage buffers

## Performance Considerations

**Vertex Buffers vs Storage Buffers:**

**Advantages of Vertex Buffers:**
- Hardware-accelerated interpolation across triangle
- Better cache locality for rasterization
- Standard pipeline, better driver optimization
- Automatic handling of vertex reuse via index buffer

**Disadvantages:**
- Limited by vertex attribute count (typical limit: 16 locations)
- Requires dynamic shader generation for different attribute sets
- More complex pipeline state management

**Expected Performance:** Vertex buffer approach should be **faster** for the transparent pass because:
1. Fragment shader receives interpolated values directly (no manual barycentric interpolation)
2. Vertex cache efficiency
3. Hardware rasterizer can optimize attribute fetching

## Open Questions

1. **Should we keep storage buffer path as fallback?**
   - Pros: Flexibility for extremely complex meshes
   - Cons: Code complexity, maintenance burden
   - Recommendation: No, vertex buffers should be sufficient

2. **Maximum attribute count?**
   - WebGPU guarantees at least 16 vertex attributes
   - Our usage: 5 (geometry) + 4 (instancing) + N (custom) ≤ 16
   - Maximum safe: ~7 custom attributes (e.g., 3 UVs + 2 colors + joints + weights)
   - Should we add validation?

3. **Do we need the attribute_indices buffer in transparent pass?**
   - No! The index buffer handles vertex indexing
   - The attribute data is already indexed correctly by vertex position
   - The indices buffer is only needed for the compute shader approach

4. **Template generation strategy?**
   - Option A: Generate separate files per attribute combination (complex)
   - Option B: Use runtime template variables (current approach, recommended)
   - Recommendation: Use existing template system with {% if %} and {% match %} blocks

## Related Files Reference

### Rust Files
- `crates/renderer/src/render_passes/material/transparent/shader/vertex.rs` - Main implementation target
- `crates/renderer/src/mesh.rs:147-194` - Render command updates
- `crates/renderer/src/mesh/buffer_info.rs` - Buffer structure definitions
- `crates/renderer/src/mesh/meta/material_opaque_meta.rs` - Reference implementation for metadata
- `crates/renderer/src/render_passes/material/transparent/shader/cache_key.rs` - Shader cache keys

### WGSL Files
- `crates/renderer/src/render_passes/shared/shader/geometry_and_transparency_wgsl/vertex/apply.wgsl` - Vertex shader
- `crates/renderer/src/render_passes/material/transparent/shader/material_transparent_wgsl/fragment.wgsl` - Fragment shader
- `crates/renderer/src/render_passes/material/transparent/shader/material_transparent_wgsl/bind_groups.wgsl` - Bind group definitions

### Reference (Opaque Pass)
- `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/compute.wgsl` - Storage buffer approach
- `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/texture_uvs.wgsl` - UV loading reference
- `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/vertex_color_attrib.wgsl` - Color loading reference

## Conclusion

The implementation is straightforward in concept but requires careful attention to:
1. **Location calculation** - Must be consistent between Rust and WGSL
2. **Template generation** - Dynamic shader creation based on attributes
3. **Buffer offset calculation** - Correctly computing byte offsets

The buffer data is already correctly formatted and interleaved, so this is primarily a matter of:
- **Exposing** that data via vertex attributes instead of storage buffers
- **Updating** the shaders to receive interpolated values directly
- **Managing** the shader variants for different attribute combinations

The architecture already supports most of this via the existing cache key system and template generation. The main work is filling in the placeholder in `vertex_buffer_layout()` and updating the shader templates.
