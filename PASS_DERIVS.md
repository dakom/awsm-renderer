The code comment at line 147 is particularly revealing:
  // This uses the verified formula that matches hardware dFdx/dFdy

  This suggests the geometric calculation was verified against hardware derivatives. Here's the brilliant solution:

  The Best Approach: Store barycentric derivatives from the geometry pass

  In the geometry fragment shader, you already have barycentric coordinates being interpolated by the hardware rasterizer (with morphs/skins already applied). You can compute the derivatives directly using
  dFdx/dFdy:

  // In geometry fragment shader
  let bary_ddx = vec2<f32>(dFdx(input.barycentric.x), dFdx(input.barycentric.y));
  let bary_ddy = vec2<f32>(dFdy(input.barycentric.x), dFdy(input.barycentric.y));
  // Store these 4 values (bary_ddx.xy, bary_ddy.xy) to a new output texture

  This gives you d(b1)/dx, d(b2)/dx, d(b1)/dy, d(b2)/dy - exactly bary_derivs!

  Then in the compute shader, you can:
  1. Read the pre-computed barycentric derivatives from the texture
  2. Skip lines 135-148 entirely (no vertex position reconstruction needed)
  3. Jump straight to the chain rule at lines 156-166

  Why this works perfectly:

  1. Hardware-accurate: The rasterizer computes barycentric coords with morphs/skins already applied
  2. Efficient: dFdx/dFdy are hardware-accelerated
  3. Direct: You get exactly what you need without reconstruction
  4. No extra vertex data: Just add one RGBA16Float texture to geometry pass outputs

  The derivatives are constant per triangle (barycentric interpolation is linear), so this should be robust even at triangle edges.
