# Ensure normals are being recalculated
  - test with some model that has morphs
  - test with some model that has skins

# Opaque material pass
- get basic lighting working
  - actually pass it in
  - point, spot, directional
- normal mapping
- occlusion
- emissive

# Transparent material pass
- like opaque
- cutoff
- complete getting alpha blend mode working again



# Get started with light culling pass

1. Write light
2. Use in opaque material pass

# get rid of 256 byte alignment for mesh meta?

- maybe only necessary for uniforms?

# Multithreading

Dynamic/Uniform storages could be SharedArrayBuffer
Requires more design/thought (don't want to expose raw manipulation)

# KHR_texture_transform support maybe
