# Next up

- AnimatedMorphCube
    - Fix 

## Approach 

High-level, approach is to keep going through gltf models, one at a time, making them each work starting with minimal and feature-tests.

As more features are added, support is added into the core engine.

## GLTF Support 

If it's supported here, corresponding core functionality is also supported

- Loaders
    - [x] document 
    - [x] buffers 
    - [x] images
- Caching
    - [x] Shaders by ShaderKey
    - [x] RenderPipelines by RenderPipelineKey
    - [x] PipelineLayouts by PipelineLayoutKey 
- Accessors
    - [x] basic 
    - [x] sparse
- Hierarchy
    - [x] transforms
    - [x] scene graph
- Geometry
    - [x] positions
    - [x] morphing
    - [x] skinning
        - up to 3 sets of joints/weights 
- Animation
    - [x] morph targets (keyable by MorphKey)
    - [x] transform targets (keyable by TransformKey)
    - [x] linear interpolation
    - [x] step interpolation
        - [ ] test 
    - [x] cubic spline interpolation
        - [ ] test 
- Extensions
    - [ ] Instancing
- Materials (TODO - fill this out as we go)
- Lighting (TODO - fill this out as we go)
- Skybox (TODO - fill this out as we go)

## Optimizations

- [x] Dynamic buffer primitives
    - Single gpu binding
    - Offset-driven
    - Allows insertions and deletions at runtime
    - Separate CPU vs. GPU updates
    - Fixed and flexible (buddy) modes
- [x] Transforms
    - One dynamic uniform bind group
    - Dirty flag
- [x] Morphs
    - One dynamic uniform bind group for weights
        - gpu gating on dirty flag
    - One dynamic storage bind group for values
        - gpu gating on dirty flag
    - Conscious shader generation
        - Number of targets -> weights -> constant override -> new shader
        - Presence of attributes -> new shader
        - Unused but present attributes do not create new shader, just 0 influence
- [x] Meshes
    - One vertex buffer
    - One index buffer
    - Gpu gating on dirty flag
- Camera
    - [x] Single uniform buffer 
    - [ ] Frustum culling

## Drawing
- [x] non-indexed
- [x] indexed
- [ ] instancing
- [ ] Early z pre-pass
- [ ] Opaque front to back
- [ ] Transparent back to front

## Animation system 
- [x] Players
    - [x] speed control
    - [x] loop control
    - [x] play/pause
        - [ ] test 
    - [x] direction 
        - [ ] test 
- [x] Clips and samplers (see gltf features for details)
- [ ] Events

## Post-processing
- [ ] SSAO
- [ ] Bloom
- [ ] FXAA
- [ ] DOF
- [ ] Tonemapping

## Demo-only

- [x] Camera
    - [x] Orbit controls 
    - [x] Orthographic
    - [x] Perspective
    - [x] Initial fit for AABB (not perfect, but good enough) 
