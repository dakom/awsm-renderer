# Transparent material pass
- like opaque
- cutoff
- complete getting alpha blend mode working again
-MAYBE: 
  - One draw call per material-kind (pbr, unlit, etc.) 
  - Fetch the material data from a the same material uniform buffer (bound w/ dynamic offset)
  - Uses Weighted Blended Order-Independent Transparency (OIT)
  - Uses the Depth buffer from (1) to discard occluded fragments w/ depth testing (but writes are off)
  - Outputs to multiple textures:
    - `oit_rgb_texture`: accumulated weighted sum of colors
    - `oit_alpha_texture`: accumulated weighted product of transparencies

### 5. Composition (Compute Shader)
- Single fullscreen compute dispatch
- Resolve OIT: Read from `oit_rgb_texture` and `oit_alpha_texture` and calculate the final transparent color.
- Composite: Blend the resolved transparent color over the `opaque_color_texture`.
- Apply TAA: Use `motion_vector_texture` to blend the current, composited frame with the previous frame's history buffer.
- Tonemapping, gamma-correction, etc.
- Outputs final resolved frame to `composite_texture`

------

At this point, can merge to main, delete this file, and continue with [ROADMAP.md](./roadmap.md)
