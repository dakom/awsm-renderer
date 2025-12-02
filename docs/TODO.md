# Transparency Pass

- template-feature-gate transprency pass (shader gen shouldn't try to load uvs, for example)
  - easy to see in ambient occlusion compare 

- vertex buffer data from non-geometry attributes (what we use for storage in opaque)
  - strip out MeshBufferCustomVertexAttributeInfo::Joints and MeshBufferCustomVertexAttributeInfo::Weights 
    - these only need to be in the storage buffer, we extract them in gltf/buffers/skin.rs and 
    - also confirm that morphs are not in attributes

------

At this point, can merge to main, delete this file, and continue with [ROADMAP.md](./roadmap.md)
