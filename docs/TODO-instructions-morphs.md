Assessment Summary

  Based on what I can see:

  Current Normal Calculation (material opaque pass)

  Location: normal.wgsl:22-36

  The current get_vertex_normal() function:
  1. Reads base normals from attribute_data buffer
  2. Interpolates them using barycentric coordinates
  3. Transforms by normal_matrix (standard model transform)

  This does NOT account for:
  - ❌ Skinning transformations (bone influences)
  - ❌ Morph target blending

  The Problem

  Your visibility pass likely:
  - ✅ Applies skinning/morphing to positions → correct geometry
  - ✅ Stores those transformed positions in visibility data

  But in the material pass:
  - ❌ Uses base, untransformed normals from original mesh
  - ❌ Normals don't match the deformed geometry

  Result:

  - Recursive Skeletons: Half black because normals point wrong direction after bone transforms
  - AnimatedMorphCube: Normals don't follow the morphed shape

  What Needs To Happen

  You need to apply the same transformations to normals that you apply to positions:

  1. For Skinning: Transform normals by bone matrices (using inverse-transpose)
  2. For Morphing: Blend normal morph targets, not just position morph targets

  The normals should be computed in the same pass where you compute positions (or stored/recomputed in the material pass with the same transformations).

  Recommendation for Tomorrow

  Check your visibility pass to see how you're handling skinning/morphing for positions, then apply the same logic to normals in the material pass. The key is that normals need to "follow" the
   deformed geometry.

  Hope this helps! Have a good night! 🌙
