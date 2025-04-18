
High-level, approach is to keep going through gltf models, one at a time, making them each work starting with minimal and feature-tests.

As more features are added, support is added into the core engine.

## Demo-only

- [ ] Camera w/ arcball controls

## Camera

- [x] Basic orthographic
- [x] Single uniform buffer 

## Optimizations

- [x] Dynamic buffer
    - Single gpu binding
    - Offset-driven
    - Allows insertions and deletions at runtime
    - Separate CPU vs. GPU updates
- [x] Transforms
    - One dynamic uniform buffer
    - Dirty flag
- [x] Morphs
    - One dynamic uniform buffer for weights
    - One-ish gpu binding for values (keyable by StorageBufferKey)
    - Conscious shader generation
        - Number of targets -> weights -> constant override -> new shader
        - Presence of attributes -> new shader
        - Unused but present attributes do not create new shader, just 0 influence
- [ ] Frustum culling

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
- Drawing
    - [x] non-indexed
    - [x] indexed
    - [ ] instancing
- Geometry
    - [x] positions
    - [x] morphing
    - [ ] skinning
- Animation
    - [x] morph targets (keyable by MorphKey)
    - [ ] transform targets (keyable by TransformKey)
        - [x] partial support so far 
    - [ ] skin targets
    - [x] linear interpolation
    - [x] step interpolation
        - [ ] test 
    - [x] cubic spline interpolation
        - [ ] test 
- Materials (TODO - fill this out as we go)
- Lighting (TODO - fill this out as we go)
- Skybox (TODO - fill this out as we go)