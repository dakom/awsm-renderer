## OVERVIEW

The renderer uses a **two-tier buffer system** that separates geometry/visibility data from custom vertex attributes:

1. **Visibility Buffers** (exploded, vertex buffers): positions, normals, tangents, triangle_index, barycentric stored per-triangle-vertex
2. **Attribute Buffers** (indexed, storage buffers): UVs, colors, joints, weights stored per-original-vertex with indexed access

Meshes can exist in both buffers simultaneously, or just one. 

The determination is done in the GLTF loading stage based on which attributes are present. Specifically, [`mesh_buffer_geometry_kind()` in `gltf/buffers/mesh.rs`](../renderer/src/gltf/buffers/mesh.rs)


This architecture is specifically designed for **deferred/visibility buffer rendering** where:
- The geometry pass writes triangle IDs to a G-buffer
- The material pass looks up custom attributes via storage buffers using those triangle IDs
- Explosion is necessary to embed per-triangle metadata (triangle_index, barycentric) in vertex data

## WHY VERTEX EXPLOSION IS NECESSARY

### The Core Reason: Triangle Identification

The explosion is **not** about fixing normals or handling hard edges - that is already handled correctly through vertex sharing. The explosion is required because of the **deferred rendering architecture**:

**The Problem**:
- Each vertex needs to carry `triangle_index` - which triangle does this vertex belong to?
- Each vertex needs `barycentric` coordinates - which corner of the triangle is this (for interpolation)?
- A shared vertex (e.g., vertex 5 used by triangles 10, 11, 12) can only store ONE `triangle_index`
- But it needs to be 10 when rendering triangle 10, 11 when rendering triangle 11, etc.
- **This is impossible with shared vertices!**

**Why These Metadata Fields Are Needed**:
1. Geometry pass writes `triangle_index` to G-buffer (or passes to next stage)
2. Material pass uses `triangle_index` to look up triangle data from storage buffers
3. Material pass uses `barycentric` to manually interpolate custom attributes (UVs, colors)
4. This allows custom attributes to stay in compact indexed form (not exploded)

**What About Normals?**:
- Normals are exploded along with positions because they're in the same vertex buffer
- But the explosion **preserves intent**:
  - Smooth edges: shared vertices with averaged normals → explosion copies same normal to all 3 triangle corners → smooth shading preserved
  - Hard edges: duplicated vertices with different normals → explosion copies respective normals → hard edges preserved
- GPU still interpolates normals correctly across each triangle
- The renderer doesn't "break" or "fix" anything - it preserves GLTF's original smooth/hard edge decisions

### What Gets Exploded and Why

**Exploded (52 bytes per vertex)**:
- Position: Must go with normals/tangents for consistent vertex structure
- Normal: Preserved from GLTF (smooth or hard edges maintained)
- Tangent: Preserved from GLTF (for normal mapping)
- **Triangle Index**: Required for deferred rendering (can't be shared!)
- **Barycentric**: Required for attribute interpolation (unique per triangle corner!)

**NOT Exploded (variable stride, stays indexed)**:
- UVs, Colors, Joints, Weights: Looked up via storage buffers in material pass
- Saves memory: cube needs 8 vertices worth of UVs, not 36
- Still gets proper interpolation via manual barycentric calculation

## POPULATION

GLTF loading happens in several stages, ultimately populating three main byte buffers:
- `index_bytes`: Triangle indices (original vertex indices, u32)
- `visibility_vertex_bytes`: Exploded visibility data (positions, normals, tangents, triangle_index, barycentric per triangle vertex)
- `attribute_vertex_bytes`: Interleaved custom attributes (UVs, colors, joints, weights per original vertex)

### Stage 1: GLTF Buffer Initialization

**Location**: `gltf/buffers.rs::GltfBuffers::new()`

For each mesh primitive:

1. **Extract or generate indices**:
   - If primitive has indices: extract via `GltfMeshBufferIndexInfo::maybe_new()`
     - Converts all index formats (u8, u16, i8, i16, u32) to u32
     - Appends to `index_bytes`
   - If primitive lacks indices: generate sequential indices via `generate_fresh_indices_from_primitive()`
     - Handles different primitive modes (Triangles, TriangleStrip, TriangleFan)
     - Converts strips/fans to triangle lists

2. **Convert to visibility buffer format**:
   - Calls `convert_to_visibility_buffer()` which populates:
     - `visibility_vertex_bytes`
     - `attribute_vertex_bytes`
     - `triangle_data_bytes`
     - `geometry_morph_bytes`
     - `material_morph_bytes`
     - `skin_joint_index_weight_bytes`
   - Returns `MeshBufferInfoWithOffset` containing all buffer offsets and metadata

### Stage 2: Visibility Buffer Conversion

**Location**: `gltf/buffers/visibility.rs::convert_to_visibility_buffer()`

This is the core transformation that converts indexed GLTF geometry into exploded visibility buffer format:

1. **Load all GLTF attributes**:
   - Extracts all semantic attributes from primitive
   - Determines vertex count from first attribute

2. **Load attribute data by kind**:
   - Via `load_attribute_data_by_kind()` in `attributes.rs`
   - Converts u16/i16 data to u32/i32 (WGSL doesn't support 16-bit)
   - Returns `BTreeMap<MeshBufferVertexAttributeInfo, Cow<[u8]>>`

3. **Ensure normals exist**:
   - If normals are missing, computes them via `compute_normals()`
   - Uses flat shading (per-triangle normals)

4. **Create visibility vertices**:
   - Via `create_visibility_vertices()` - see Stage 3 below
   - Writes to `visibility_vertex_bytes`

5. **Pack vertex attributes**:
   - Via `pack_vertex_attributes()` - see Stage 4 below
   - Writes to `attribute_vertex_bytes`
   - Returns list of `MeshBufferVertexAttributeInfo` describing the layout

6. **Pack triangle data**:
   - Via `pack_triangle_data()` in `triangle.rs`
   - Writes original vertex indices (3 u32 per triangle)
   - Writes to `triangle_data_bytes`

7. **Handle morph targets**:
   - Converts position morphs (geometry) and normal/tangent morphs (material)
   - Writes to respective morph buffers

8. **Handle skinning**:
   - Converts joint indices and weights
   - Writes to `skin_joint_index_weight_bytes`

### Stage 3: Visibility Vertex Creation (Triangle Explosion)

**Location**: `gltf/buffers/visibility.rs::create_visibility_vertices()`

This function performs the critical "explosion" of shared vertices into per-triangle-vertex data:

1. **Extract triangle indices**:
   - Via `extract_triangle_indices()` in `index.rs`
   - Returns `Vec<[usize; 3]>` where each element is [v0, v1, v2]
   - These indices reference the ORIGINAL per-vertex attribute data

2. **Process each triangle**:
   - Adjusts winding order based on `front_face`
   - Assigns barycentric coordinates: [1,0], [0,1], [0,0] (adjusted for winding)
   - For each of the 3 vertices in the triangle:
     - Looks up position from original positions buffer using vertex index
     - Looks up normal from original normals buffer (preserves GLTF smooth/hard edges!)
     - Looks up tangent from original tangents buffer, or default to [0,0,0,1]
     - Writes complete vertex to `visibility_vertex_bytes`:
       - Position (vec3<f32>): 12 bytes
       - Triangle Index (u32): 4 bytes - **unique per triangle**
       - Barycentric (vec2<f32>): 8 bytes - **unique per corner**
       - Normal (vec3<f32>): 12 bytes - **copied from original GLTF vertex**
       - Tangent (vec4<f32>): 16 bytes - **copied from original GLTF vertex**
       - **Total: 52 bytes per vertex**

**Key insight**: This creates 3 complete vertices per triangle, even if those vertices were shared in the original mesh. A cube with 8 vertices and 12 triangles becomes 36 vertices (12 * 3). The normals are copied from GLTF's original data, preserving smooth/hard edge decisions.

### Stage 4: Custom Attribute Packing

**Location**: `gltf/buffers/attributes.rs::pack_vertex_attributes()`

This function packs the non-visibility attributes (UVs, colors, joints, weights) in interleaved format:

1. **Filter to custom attributes only**:
   - Only includes attributes where `is_custom_attribute()` returns true
   - Excludes positions, normals, tangents (those are in visibility buffer)

2. **Determine vertex count**:
   - Validates all attributes have same vertex count
   - Uses attribute stride (`vertex_size()`) to compute count
   - Errors if attributes have mismatched lengths

3. **Interleave attributes per-vertex**:
   - For each vertex (0..vertex_count):
     - For each attribute (in BTreeMap order):
       - Copy one vertex worth of data for that attribute
       - Append to `vertex_attribute_bytes`

**Attribute ordering**: BTreeMap orders by:
- Primary: Attribute type (Positions < Normals < Tangents < Colors < TexCoords < Joints < Weights)
- Secondary: Count index (e.g., TEXCOORD_0 < TEXCOORD_1 < TEXCOORD_2)

**Example layout** for a vertex with TEXCOORD_0 and TEXCOORD_1:
```
[uv0_x(f32)][uv0_y(f32)][uv1_x(f32)][uv1_y(f32)] = 16 bytes per vertex
```

### Stage 5: Slicing and Passing to Meshes

**Location**: `gltf/populate/mesh.rs::populate_gltf_primitive()`

After all primitives are processed and the byte buffers are populated, this function creates slices for each primitive and passes them to the mesh system:

1. **Get primitive buffer info**:
   - Retrieves cached `MeshBufferInfoWithOffset` from `ctx.data.buffers.meshes`
   - Contains all offsets and sizes for this specific primitive

2. **Create visibility data slice**:
   ```rust
   let visibility_data_start = primitive_buffer_info.vertex.offset;
   let visibility_data_end = visibility_data_start + MeshBufferVertexInfo::from(...).size();
   let visibility_data = &ctx.data.buffers.visibility_vertex_bytes[visibility_data_start..visibility_data_end];
   ```

3. **Create attribute data slice**:
   ```rust
   let attribute_data_start = primitive_buffer_info.triangles.vertex_attributes_offset;
   let attribute_data_end = attribute_data_start + primitive_buffer_info.triangles.vertex_attributes_size;
   let attribute_data = &ctx.data.buffers.attribute_vertex_bytes[attribute_data_start..attribute_data_end];
   ```

4. **Create attribute index slice**:
   ```rust
   let attribute_index_start = primitive_buffer_info.triangles.vertex_attribute_indices.offset;
   let attribute_index_end = attribute_index_start + primitive_buffer_info.triangles.vertex_attribute_indices.total_size();
   let attribute_index = &ctx.data.buffers.index_bytes[attribute_index_start..attribute_index_end];
   ```

5. **Insert into mesh system**:
   ```rust
   self.meshes.insert(
       mesh,
       &self.materials,
       &self.transforms,
       visibility_data,      // Exploded vertex data
       attribute_data,       // Interleaved custom attributes
       attribute_index,      // Original triangle indices
   )?
   ```

### Stage 6: Mesh System Storage

**Location**: `mesh/meshes.rs::insert()`

The mesh system receives the slices and stores them in dynamic GPU buffers:

1. **Generate visibility index**:
   ```rust
   let mut visibility_index = Vec::new();
   for i in 0..buffer_info.vertex.count {
       visibility_index.extend_from_slice(&(i as u32).to_le_bytes());
   }
   ```
   - Creates sequential indices: 0, 1, 2, 3, 4, 5, ...
   - One index per exploded vertex
   - Used with the exploded visibility vertex buffer

2. **Update dynamic buffers**:
   - `visibility_index_buffers`: Sequential indices for drawing
   - `visibility_data_buffers`: Exploded visibility vertex data (52 bytes per vertex)
   - `attribute_index_buffers`: Original triangle indices (12 bytes per triangle)
   - `attribute_data_buffers`: Interleaved custom attributes (variable stride per vertex)

3. **Mark buffers as dirty**:
   - All buffers marked dirty after update
   - Will trigger GPU upload in next `write_gpu()` call

## INTERLEAVING

### Visibility / Geometry Attributes

**Visibility Vertex Buffer** (`visibility_vertex_bytes` → `visibility_data_buffers` → GPU vertex buffer):

- **Storage Type**: Vertex buffer (GPU_VERTEX usage)
- **Access Pattern**: Direct vertex shader input
- **Layout**: Fully interleaved, exploded per-triangle-vertex
- **Stride**: 52 bytes per vertex (constant)
- **Format**:
  | Attribute | Type | Size | Offset | Shader Location | Purpose |
  |-----------|------|------|--------|-----------------|---------|
  | Position | vec3<f32> | 12 bytes | 0 | 0 | Vertex position |
  | Triangle Index | u32 | 4 bytes | 12 | 1 | Identifies which triangle (for G-buffer/deferred lookup) |
  | Barycentric | vec2<f32> | 8 bytes | 16 | 2 | Triangle corner ID (for manual interpolation) |
  | Normal | vec3<f32> | 12 bytes | 24 | 3 | Surface normal (from GLTF, preserves smooth/hard edges) |
  | Tangent | vec4<f32> | 16 bytes | 36 | 4 | Tangent space (for normal mapping) |

- **Data Characteristics**:
  - Each triangle has 3 complete vertices (exploded)
  - Vertices are NOT shared between triangles
  - Example: A cube (8 vertices, 12 triangles) becomes 36 vertices (12 * 3)
  - Triangle Index and Barycentric cannot be shared - this is why explosion is necessary

- **Created by**: `create_visibility_vertices()` in `visibility.rs`

**Visibility Index Buffer** (`visibility_index_buffers` → GPU index buffer):

- **Storage Type**: Index buffer (GPU_INDEX usage)
- **Access Pattern**: Index buffer for drawing
- **Layout**: Sequential u32 values
- **Format**: 0, 1, 2, 3, 4, 5, 6, ...
- **Purpose**: Simple sequential indexing for exploded triangles
- **Generated by**: `meshes.rs::insert()`

### Custom Attributes

**Attribute Data Buffer** (`attribute_vertex_bytes` → `attribute_data_buffers` → GPU storage buffer):

- **Storage Type**: Storage buffer (GPU_STORAGE usage)
- **Access Pattern**: Indexed lookup via shader in material pass
- **Layout**: Fully interleaved per-vertex, using ORIGINAL vertex indices (NOT exploded)
- **Stride**: Variable (depends on which attributes exist)
- **Format**: For each vertex, all its custom attributes in sequence

**Attribute ordering** (via BTreeMap):
  1. Colors (COLOR_0, COLOR_1, ...)
  2. TexCoords (TEXCOORD_0, TEXCOORD_1, ...)
  3. Joints (JOINTS_0, JOINTS_1, ...)
  4. Weights (WEIGHTS_0, WEIGHTS_1, ...)

**Common examples**:

Example 1: Mesh with TEXCOORD_0 only
```
Stride: 8 bytes
Per vertex: [uv_x(f32)][uv_y(f32)]
```

Example 2: Mesh with TEXCOORD_0 and TEXCOORD_1
```
Stride: 16 bytes
Per vertex: [uv0_x(f32)][uv0_y(f32)][uv1_x(f32)][uv1_y(f32)]
```

Example 3: Mesh with COLOR_0 (RGB) and TEXCOORD_0
```
Stride: 20 bytes
Per vertex: [r(f32)][g(f32)][b(f32)][uv_x(f32)][uv_y(f32)]
```

Example 4: Skinned mesh with JOINTS_0, WEIGHTS_0, and TEXCOORD_0
```
Stride: 40 bytes
Per vertex: [j0(u32)][j1(u32)][j2(u32)][j3(u32)][w0(f32)][w1(f32)][w2(f32)][w3(f32)][uv_x(f32)][uv_y(f32)]
```

- **Data Characteristics**:
  - Uses ORIGINAL vertex count (not exploded)
  - Vertices ARE shared between triangles
  - Example: A cube's 8 vertices have 8 entries in this buffer
  - Accessed via `attribute_index_buffers` to get the right vertex data for each triangle

- **Attributes Included**:
  - Colors (COLOR_n)
  - Texture Coordinates (TEXCOORD_n)
  - Joint Indices (JOINTS_n)
  - Joint Weights (WEIGHTS_n)

- **Attributes Excluded**:
  - Positions (in visibility buffer)
  - Normals (in visibility buffer)
  - Tangents (in visibility buffer)

- **Created by**: `pack_vertex_attributes()` in `attributes.rs`

**Attribute Index Buffer** (`index_bytes` → `attribute_index_buffers` → GPU storage buffer):

- **Storage Type**: Storage buffer (GPU_STORAGE usage)
- **Access Pattern**: Indexed lookup via shader (reads triangle data, uses indices to fetch attributes)
- **Layout**: Triangle list indices (3 u32 per triangle)
- **Format**: u32 indices
- **Purpose**: Maps from triangle vertex to original vertex in attribute data buffer
- **Data Characteristics**:
  - Contains ORIGINAL vertex indices (before explosion)
  - Each triangle has 3 indices pointing to attribute data
  - Example: Triangle 0 might have indices [0, 1, 2], Triangle 1 might have [0, 2, 3] (sharing vertices)

**Triangle Data Buffer** (`triangle_data_bytes`):

- **Storage Type**: Storage buffer (GPU_STORAGE usage)
- **Access Pattern**: Indexed lookup via triangle index
- **Layout**: Per-triangle data
- **Stride**: 12 bytes per triangle
- **Format**: 3 u32 vertex indices per triangle
- **Purpose**: Provides original vertex indices for attribute lookup
- **Note**: Currently appears to duplicate the attribute index buffer data. May be extended in the future with per-triangle material info.

## GPU USAGE

### Geometry Pass (Opaque - Deferred Rendering)

**Location**: `mesh.rs::push_geometry_pass_commands()`

**Rendering Approach**: Deferred rendering with visibility buffer

Vertex inputs:
- Slot 0: Visibility vertex buffer (locations 0-4)
  - Bound at offset from `visibility_data_buffer_offset()`
  - Contains: position, triangle_index, barycentric, normal, tangent
- Slot 1: Instance transform buffer (if instanced)
  - Bound at offset from `instances.transform_buffer_offset()`

Index buffer:
- Visibility index buffer (sequential 0,1,2,3,...)
  - Bound at offset from `visibility_index_buffer_offset()`

Bind groups:
- Bind group 2: Mesh meta (dynamic offset to geometry buffer offset)

**Vertex Shader Flow**:
1. Reads exploded vertex data (position, normal, tangent, triangle_index, barycentric)
2. Transforms geometry
3. Outputs triangle_index (flat) to fragment shader

**Fragment Shader Flow**:
1. Receives triangle_index from vertex shader
2. Writes triangle_index to G-buffer

**Material Pass Flow** (separate pass):
1. Reads triangle_index from G-buffer for each pixel
2. Uses triangle_index to look up original vertex indices from storage buffer
3. Uses barycentric coordinates to manually interpolate UVs/colors from storage buffer
4. Performs lighting calculations with interpolated attributes

**Why This Works Without Exploded Custom Attributes**:
- Material pass has access to triangle_index from G-buffer
- Can look up any data via storage buffers
- Manual interpolation using barycentric coordinates
- Custom attributes stay compact (8 vertices for cube, not 36)

### Material Pass - Transparent (WIP - Forward Rendering)

**Location**: `mesh.rs::push_material_transparent_pass_commands()`

**⚠️ TRANSPARENCY PASS IS WORK IN PROGRESS**

**Rendering Approach**: Traditional forward rendering (not deferred)

**Why Different from Opaque**:
- Transparency requires blending, which doesn't work with deferred rendering
- Must render in sorted order with immediate lighting calculations
- No G-buffer, no triangle_index available in fragment shader
- Cannot use storage buffer lookups with manual interpolation

**Recommended Implementation**:
Use traditional indexed rendering with a single interleaved vertex buffer:

```rust
// Single vertex buffer with ALL attributes (not exploded)
VertexBufferLayout {
    array_stride: 56, // position(12) + normal(12) + tangent(16) + uv(8) + color(12) = variable
    step_mode: None,
    attributes: vec![
        VertexAttribute { format: Float32x3, offset: 0,  location: 0 },  // position
        VertexAttribute { format: Float32x3, offset: 12, location: 1 },  // normal
        VertexAttribute { format: Float32x4, offset: 24, location: 2 },  // tangent
        VertexAttribute { format: Float32x2, offset: 40, location: 3 },  // uv
        // ... more as needed
    ],
}
```

**Memory Comparison** (cube with 2 UV sets):

Current exploded approach (if implemented):
- Visibility buffer: 36 vertices × 52 bytes = 1,872 bytes
- Custom attributes exploded: 36 vertices × 16 bytes = 576 bytes
- Total: **2,448 bytes**
- Contains unnecessary data: triangle_index and barycentric not needed for forward rendering

Recommended traditional approach:
- Single interleaved buffer: 8 vertices × 56 bytes = 448 bytes
- Index buffer: 36 indices × 4 bytes = 144 bytes
- Total: **592 bytes (4x smaller!)**
- No wasted data: only what's needed for forward rendering

**Why This Makes More Sense**:
- Triangle explosion is specifically for deferred rendering (triangle_index + barycentric)
- Forward rendering doesn't need these metadata fields
- GPU hardware interpolation is simpler and faster than manual interpolation
- Significantly less memory usage
- Traditional approach is well-tested and efficient

**Current State**:
- Uses same visibility vertex buffer as geometry pass (Slot 0, locations 0-4)
- Instancing in Slot 1 (locations 5-8) if enabled
- Custom attributes would go in Slot 2/3 (locations 5-* or 8-*)
- `transparent/shader/vertex.rs` vertex buffer layout is incomplete:
  - `array_stride: 0` (placeholder)
  - `attributes: vec![]` (placeholder)

## KEY ARCHITECTURAL DECISIONS

### Why Vertex Explosion Is Required (For Opaque/Deferred)

**The Core Problem**:
Deferred rendering with visibility buffers requires embedding per-triangle metadata in vertex data:
- **triangle_index**: Which triangle is this? (needed for G-buffer storage and later lookup)
- **barycentric**: Which corner of the triangle? (needed for manual attribute interpolation)

**Why Shared Vertices Don't Work**:
- GLTF vertex 5 might be used by triangles 10, 11, and 12
- When rendering triangle 10, vertex 5 needs triangle_index = 10
- When rendering triangle 11, vertex 5 needs triangle_index = 11
- A single shared vertex can only store ONE value
- **Solution**: Explode vertices so each triangle corner has its own vertex with unique metadata

**What About Normals**:
- Normals get exploded along with positions (they're in the same vertex buffer)
- But explosion **preserves GLTF's original intent**:
  - Smooth edges: GLTF shared vertices → same normal copied to all 3 corners → smooth shading preserved
  - Hard edges: GLTF duplicated vertices → different normals → hard edges preserved
- GPU interpolation works normally across each triangle
- The renderer doesn't "fix" GLTF data - it preserves smooth/hard edge decisions

**Why Custom Attributes Stay Indexed**:
- Material pass has triangle_index from G-buffer
- Can look up any data from storage buffers
- Uses barycentric coordinates for manual interpolation
- Saves memory: cube needs 8 UVs, not 36

### Why Transparency Should NOT Use Explosion

**Different Rendering Approach**:
- Opaque: Deferred rendering → needs triangle_index/barycentric → requires explosion
- Transparent: Forward rendering → no G-buffer, no triangle_index needed → traditional approach better

**Memory Efficiency**:
- Exploded (if implemented): 2,448 bytes for cube with UVs (includes useless triangle_index/barycentric)
- Traditional indexed: 592 bytes for same cube (4x smaller, only necessary data)

**Simplicity**:
- Forward rendering: one vertex buffer, hardware interpolation, standard approach
- No need for storage buffer lookups or manual interpolation
- Cleaner shader code

### Attribute Ordering

**Why BTreeMap ordering matters**:
- Ensures consistent interleaving across all meshes
- Allows shader code to know attribute offsets without per-mesh configuration
- Order: Positions, Normals, Tangents, Colors, TexCoords, Joints, Weights
- Within each category: sorted by count (COLOR_0 before COLOR_1, etc.)

**Example**: Two meshes, one with TEXCOORD_0 only, another with COLOR_0 + TEXCOORD_0:
- Mesh 1: [TEXCOORD_0] = 8 bytes stride
- Mesh 2: [COLOR_0][TEXCOORD_0] = 20 bytes stride
- Both use same attribute ordering, just different strides

### Index Buffer Separation

Two separate index buffers with different purposes:

1. **Visibility Index** (`visibility_index_buffers`):
   - Sequential: 0, 1, 2, 3, ...
   - Used with exploded visibility vertices
   - Index buffer (VERTEX usage)
   - Simple and predictable

2. **Attribute Index** (`attribute_index_buffers`):
   - Original triangle indices with vertex sharing
   - Used with non-exploded attribute data
   - Storage buffer (STORAGE usage)
   - Allows vertex reuse for attributes

## DATA FLOW SUMMARY

```
GLTF File
    ↓
[Load raw buffers]
    ↓
For each primitive:
    ↓
[Extract/Generate Indices] → index_bytes (u32 per index)
    ↓
[Convert to Visibility Buffer]
    ↓
    ├→ [Create Visibility Vertices] → visibility_vertex_bytes (52 bytes per exploded vertex)
    │   - Embeds triangle_index and barycentric (required for deferred rendering)
    │   - Copies normals from GLTF (preserves smooth/hard edges)
    ├→ [Pack Vertex Attributes] → attribute_vertex_bytes (variable stride per original vertex)
    │   - NOT exploded, stays indexed
    │   - Accessed via storage buffers in material pass
    ├→ [Pack Triangle Data] → triangle_data_bytes (12 bytes per triangle)
    ├→ [Convert Morph Targets] → geometry_morph_bytes, material_morph_bytes
    └→ [Convert Skin] → skin_joint_index_weight_bytes
    ↓
[Store offsets in MeshBufferInfoWithOffset]
    ↓
[Slice buffers per primitive]
    ↓
    ├→ visibility_data slice
    ├→ attribute_data slice
    └→ attribute_index slice
    ↓
[meshes.insert()]
    ↓
    ├→ Generate sequential visibility index
    ├→ Update visibility_data_buffers (DynamicStorageBuffer)
    ├→ Update visibility_index_buffers (DynamicStorageBuffer)
    ├→ Update attribute_data_buffers (DynamicStorageBuffer)
    └→ Update attribute_index_buffers (DynamicStorageBuffer)
    ↓
[write_gpu()]
    ↓
    ├→ visibility_data_gpu_buffer (VERTEX usage)
    ├→ visibility_index_gpu_buffer (INDEX usage)
    ├→ attribute_data_gpu_buffer (STORAGE usage)
    └→ attribute_index_gpu_buffer (STORAGE usage)
    ↓
GPU Rendering
    ↓
    ├→ Geometry Pass (Opaque): Deferred rendering with exploded visibility buffer
    └→ Transparent Pass (WIP): Should use traditional forward rendering with non-exploded unified buffer
```

## NOTES

- All buffer data uses **little-endian byte order** per glTF specification
- Vertex data validation happens during packing (errors if mismatched attribute lengths)
- Dynamic buffers automatically resize as needed when adding meshes
- Buffer resizes trigger bind group recreation for storage buffers
- The `MeshBufferVertexInfo::BYTE_SIZE` constant (52) is critical for visibility buffer calculations
- Vertex explosion is specifically for deferred rendering - forward-rendered transparent objects should use traditional indexed approach
