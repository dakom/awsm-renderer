# Delete gradient mip mode

----
okay, do you think if I copy/pasted that file to you in another session you'd be able to pick it up? (assuming I also prepared by loading a cubemap for environment sampling)

⏺ Yes, absolutely! If you copy/paste that TODO.md in a new session along with information about your cubemap setup, I should be able to pick it up and help you implement the proper lighting.

  To make it even smoother, when you start the new session, also provide:

  1. The TODO.md file (which you'll have)
  2. Info about your cubemap:
    - Where it's loaded/stored
    - Format (HDR? What texture type?)
    - How to access it in the shader (binding group/location)
  3. Context about the codebase structure - maybe just say "this is a WebGPU deferred renderer, compute shader does lighting in compute.wgsl, BRDF is in brdf.wgsl"

  With that context, I'll be able to jump right into implementing step 1 (separating direct from IBL in BRDF) and step 2 (using your real cubemap for IBL sampling).

  The TODO.md is pretty detailed, so it should work well as a handoff document!
  ----
# Fix disappearing textures

# Opaque material pass
- get basic lighting working
  - actually pass it in
  - point, spot, directional
- IBL
  - metallic/roughness on spheres should look better
- normal mapping
- occlusion
- emissive

# Transparent material pass
- like opaque
- cutoff
- complete getting alpha blend mode working again

# Ensure normals are being recalculated
  - test with some model that has morphs
  - test with some model that has skins


# Get started with light culling pass

1. Write light
2. Use in opaque material pass

# get rid of 256 byte alignment for mesh meta?

- maybe only necessary for uniforms?

# Multithreading

Dynamic/Uniform storages could be SharedArrayBuffer
Requires more design/thought (don't want to expose raw manipulation)

# KHR_texture_transform support maybe
