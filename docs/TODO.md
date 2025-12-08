- skin and morph data should be index, with a bit of redirection - not exploded
- skin and morph data should not exist in custom attributes at all (only from gltf side, not mesh)
  - strip out MeshBufferCustomVertexAttributeInfo::Joints and MeshBufferCustomVertexAttributeInfo::Weights 
    - these only need to be in the storage buffer, we extract them in gltf/buffers/skin.rs and 
    - also confirm that morphs are not in attributes

------

At this point, can merge to main, delete this file, and continue with [ROADMAP.md](./roadmap.md)
