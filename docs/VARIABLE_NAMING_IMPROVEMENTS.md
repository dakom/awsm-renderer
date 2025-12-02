# Variable Naming Improvements to Avoid Confusion

## The Problem

Both `GeometryMeshMeta` and `MaterialMeshMeta` have a field called `material_offset`, but they mean **completely different things**:

- `GeometryMeshMeta.material_offset` → offset into the **MaterialMeshMeta buffer** (metadata)
- `MaterialMeshMeta.material_offset` → offset into the **PBR Materials buffer** (actual data)

Having the same name for fields that point to different buffers caused the transparency bug.

## Recommended Renaming

### Option 1: Rename the GeometryMeshMeta field (Minimal Change)

**Rust side:**
```rust
// In GeometryMeshMeta struct
pub struct GeometryMeshMeta {
    // ... other fields ...
    pub material_meta_offset: u32,  // Was: material_offset
}
```

**WGSL side:**
```wgsl
struct GeometryMeshMeta {
    // ... other fields ...
    material_meta_offset: u32,  // Was: material_offset
}
```

This makes it crystal clear that this offset points to **metadata**, not the actual material.

### Option 2: Be Fully Explicit (Better)

**Rust side:**
```rust
// In GeometryMeshMeta struct
pub struct GeometryMeshMeta {
    // ... other fields ...
    pub material_mesh_meta_offset: u32,  // Was: material_offset
}
```

**WGSL side:**
```wgsl
struct GeometryMeshMeta {
    // ... other fields ...
    material_mesh_meta_offset: u32,  // Was: material_offset
}
```

This explicitly names what buffer it points to: the **MaterialMeshMeta** buffer.

### Option 3: Use Pass-Based Naming (Alternative)

**Rust side:**
```rust
// In GeometryMeshMeta struct
pub struct GeometryMeshMeta {
    // ... other fields ...
    pub material_pass_meta_offset: u32,  // Was: material_offset
}
```

This emphasizes it points to metadata for the material pass.

## Recommended Action

**Use Option 2** (`material_mesh_meta_offset`) - it's the most explicit and self-documenting.

### Files to Update

1. **Rust:**
   - `crates/renderer/src/mesh/meta/geometry_meta.rs` - struct definition
   - `crates/renderer/src/mesh/meta/geometry_meta.rs` - serialization code
   - Any code that reads this field

2. **WGSL:**
   - `crates/renderer/src/render_passes/shared/shader/geometry_and_transparency_wgsl/vertex/meta.wgsl` - struct definition
   - `crates/renderer/src/render_passes/material/transparent/shader/material_transparent_wgsl/fragment.wgsl` - reading the field
   - Any other shaders that read from GeometryMeshMeta

3. **Update the existing comment** in the WGSL file:
```wgsl
// Before:
// this is not the offset of the material
// it's the offset of the mesh_meta data in the material *pass*
material_offset: u32,

// After (no comment needed if name is clear):
material_mesh_meta_offset: u32,
```

## General Naming Principle

When you have offsets/pointers to different buffer types, include the **buffer type name** in the field name:

- ✅ `material_mesh_meta_offset` - points to MaterialMeshMeta buffer
- ✅ `material_offset` - points to Material buffer
- ✅ `transform_offset` - points to Transform buffer
- ❌ `material_offset` - ambiguous when you have multiple material-related buffers
