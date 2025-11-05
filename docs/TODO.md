# Transparent material pass
- like opaque
- cutoff
- complete getting alpha blend mode working again

# Fix perspective projection skybox

# Barycentric derivatives via dFdx/dFdy

# discard and single-sample the non-depth targets in the geometry pass

# Do DEBUG_MIPMAPS.md



# Switch to octahedral encoding for normals+tangent

Normal+Tangent (skinned/morphed): compact MS target(s) so you aren’t writing 4× big float buffers:

Use octahedral encoding for unit vectors.

Put normal in RG8UNORM (two channels), and tangent in BA8UNORM (two channels) of a single RGBA8UNORM attachment; put handedness/sign in one spare bit (e.g., flip a channel by 0.5 threshold) or in another small attachment if you prefer clarity.

This way you’ve avoided giant rgba16f/rgba32f MS textures for TBN, but you still get hardware-interpolated, skinned/morphed TBN at the fragment stage.

------

At this point, can merge to main, delete this file, and continue with [ROADMAP.md](./roadmap.md)
