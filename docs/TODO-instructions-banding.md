⏺ Continuation Instructions for Tomorrow

  Current State Summary

  What Works ✅

  - Real IBL texture sampling (irradiance + prefiltered environment + BRDF LUT)
  - Split BRDF (direct lighting separate from IBL)
  - Fixed 4x IBL multiplication bug
  - Compute shader compatibility (using textureSampleLevel)

  What Needs Fixing ❌

  1. BRDF LUT has incorrect values causing dark rings on rough+metallic spheres
  2. Currently using analytic approximation as workaround (see brdf.wgsl:158-163)
  3. Need better IBL environment for testing (current placeholder may have issues)

  ---
  Priority 1: Create Simple Sky IBL Environment

  Before fixing the BRDF LUT, set up a proper test environment so you can see real IBL behavior.

  Goal: Create a simple gradient sky (bright top, darker bottom) for IBL testing.

  Where to look:
  - Check how you're currently loading IBL textures (search for set_ibl or Ibl::new)
  - Create a simple procedural sky cubemap:
    - Top face (Y+): Bright blue/white (simulate sky)
    - Bottom face (Y-): Darker (simulate ground)
    - Side faces: Gradient from top to bottom
  - Generate both irradiance and prefiltered environment maps from this

  Why: A simple, known environment will help you verify if lighting looks correct vs. artifact from bad environment data.

  ---
  Priority 2: Fix BRDF LUT Generation

  The BRDF LUT shader has issues causing dark rings. Here's what we tried and what might still be wrong:

  What We Already Fixed

  - ✅ Changed importance_sample_ggx(xi, roughness) → importance_sample_ggx(xi, alpha) where alpha = roughness * roughness
  - ✅ Updated geometry term to match main BRDF: k = ((a + 1.0) * (a + 1.0)) * 0.125
  - ✅ Pass alpha instead of roughness to geometry_smith()

  What Might Still Be Wrong

  File: crates/renderer-core/src/brdf_lut/shader.wgsl

  Issue: The BRDF integration might have numerical issues or incorrect Monte Carlo setup.

  Debug Steps:

  1. Verify the BRDF LUT visually:
  // In compute.wgsl, add this debug:
  let uv = vec2<f32>(f32(coords.x) / f32(screen_dims.x), f32(coords.y) / f32(screen_dims.y));
  let brdf_lut = textureSampleLevel(brdf_lut_tex, brdf_lut_sampler, uv, 0.0).rg;
  color = vec3<f32>(brdf_lut.x, brdf_lut.y, 0.0);

  1. Expected:
    - Top-left (rough + grazing): Yellow/orange (high bias)
    - Top-right (rough + facing): Orange/red
    - Bottom-left (smooth + grazing): Green (high bias, low scale)
    - Bottom-right (smooth + facing): Bright red (high scale)

  If you see: Black areas, bands, or discontinuities → BRDF LUT generation is broken
  2. Compare against reference:
    - Download a reference BRDF LUT from https://learnopengl.com/PBR/IBL/Specular-IBL or generate one with https://github.com/dariomanesku/cmftStudio
    - Load it directly instead of generating
    - See if dark rings disappear
  3. Check importance sampling:
    - The importance_sample_ggx function (line 25-33) might have issues
    - Verify the PDF and hemisphere sampling are correct
    - Compare against Epic's reference implementation
  4. Increase sample count:
    - Try changing sample_count: u32 = 1024u → 4096u (line 51)
    - If dark rings reduce, it's a sampling/noise issue

  ---
  Priority 3: Re-enable BRDF LUT Texture

  File: crates/renderer/src/render_passes/material/shared/shader/pbr_shared_wgsl/lighting/brdf.wgsl

  Current state (lines 158-167):
  // TEST: Use analytic approximation instead of BRDF LUT (Karis 2013)
  let a = roughness;
  let r = max(1.0 - a, 0.0);
  let scale = mix(r, 1.0, pow(1.0 - n_dot_v_clamped, 5.0));
  let bias = (1.0 - r) * pow(1.0 - n_dot_v_clamped, 5.0);
  let Fs_indir = prefiltered * (F0 * scale + vec3<f32>(bias));

  // ORIGINAL: Use BRDF LUT texture
  // let brdf_lut    = sampleBRDFLUT(n_dot_v_clamped, roughness, brdf_lut_tex, brdf_lut_sampler);
  // let Fs_indir    = prefiltered * (F0 * brdf_lut.x + brdf_lut.y);

  Once BRDF LUT is fixed: Comment out the analytic approximation and uncomment the texture sampling.

  ---
  Testing Checklist

  After fixes, verify on MetalRoughSpheres model:

  - No dark rings on any sphere
  - Smooth gradient from center to edges (Fresnel effect)
  - Rougher spheres (right) look more matte/diffuse
  - Metallic spheres (top) show clear environment reflections
  - Lighting transitions smoothly as you rotate camera
  - No sudden discontinuities or bands

  ---
  Additional Notes

  About the "Darker Hemisphere"

  The current behavior (darker on one side) might be physically correct:
  - Spheres naturally have lighting variation across their surface
  - Fresnel effect makes grazing angles darker (less diffuse)
  - This is how real spheres look with IBL

  However, if it looks too dark:
  - Your IBL environment might not be bright enough
  - Enable back-face culling so you don't render the "back" of spheres
  - Add a minimum ambient term (but this reduces physical accuracy)

  Files to Focus On

  1. crates/renderer-core/src/brdf_lut/shader.wgsl - BRDF LUT generation
  2. crates/renderer/src/render_passes/material/shared/shader/pbr_shared_wgsl/lighting/brdf.wgsl - IBL calculation
  3. Wherever you load/set IBL textures - Create sky environment

  ---
  Quick Reference: Key Issues Found Tonight

  | Issue                             | Location                   | Fix Applied                               |
  |-----------------------------------|----------------------------|-------------------------------------------|
  | IBL stub functions                | brdf.wgsl:43-77            | ✅ Replaced with real texture sampling     |
  | 4x IBL multiplication             | compute.wgsl:313-337       | ✅ Split into brdf_direct() + brdf_ibl()   |
  | Wrong mip for importance sampling | brdf_lut/shader.wgsl:57    | ✅ Pass alpha instead of roughness         |
  | Wrong geometry term               | brdf_lut/shader.wgsl:34-40 | ✅ Match main BRDF formula                 |
  | textureSample in compute shader   | brdf.wgsl:50,78            | ✅ Changed to textureSampleLevel           |
  | Dark rings on rough spheres       | brdf.wgsl:158-167          | ⚠️ Workaround with analytic approximation |

  Good luck tomorrow! 🚀
