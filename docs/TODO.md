# Opaque material pass
- vertex colors (last of the GLTF attributes we need to suport)
  - https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh-primitive
- base color compare
- occlusion test
- emissive test

# Bring back TAA
- Very obvious on MorphStressTest
- Also lines on EmissiveStrength

# Transparent material pass
- like opaque
- cutoff
- complete getting alpha blend mode working again


------

At this point, can merge to main, delete this file, and continue with [ROADMAP.md](./roadmap.md)
