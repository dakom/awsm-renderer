# Next up

- TAA for temporal and edge aliasing

- materials cont'd
    - normal texture
    - metal roughness texture

- IBL

- Is it right that specular isn't moving across sphere test when camera moves?


- fix "dev-release" mode (just base path?)

- Initial allocation?
    - derive from gltf?

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
        - multiple sets (as many as fit in attribute slots)
- Animation
    - [x] morph targets (keyable by MorphKey)
    - [x] transform targets (keyable by TransformKey)
    - [x] linear interpolation
    - [x] step interpolation
    - [x] cubic spline interpolation
- Extensions
    - [x] EXT_mesh_gpu_instancing
    - [ ] KHR_materials_unlit
    - [ ] KHR_lights_punctual
        - [ ] Directional
        - [ ] Point
        - [ ] Spot
    - [ ]  more at https://github.com/KhronosGroup/glTF/blob/main/extensions/README.md#ratified-khronos-extensions 
- Materials
    - [ ] PBR metallic-roughness
        - [x] base color
        - [ ] metallic
        - [ ] roughness
        - [ ] normal
        - [ ] occlusion
        - [ ] emissive
    - [x] mipmaps
- Lighting (TODO - fill this out as we go)
- Skybox (TODO - fill this out as we go)

## Optimizations

- [x] Dynamic buffer primitives
    - Single gpu binding
    - Offset-driven
    - Allows insertions and deletions at runtime
    - Separate CPU vs. GPU updates
    - Fixed and flexible (buddy) modes
    - [ ] Evaluate if mapped buffers helps here
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
- [x] Instancing
    - One dynamic uniform bind group for transforms
    - Gpu gating on dirty flag
- Camera
    - [x] Single uniform buffer 
    - [ ] Frustum culling

## Drawing
- [x] non-indexed
- [x] indexed
- [x] instancing
- [ ] Early z pre-pass
- [x] Opaque front to back
- [x] Transparent back to front

## Textures
- [x] 2D textures
- [ ] 3D textures
- [x] Mipmaps (port https://github.com/JolifantoBambla/webgpu-spd)

## Skybox
- [ ] Cubemap

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
- [x] Basic render-texture support
- [x] Tonemapping
- [ ] SSAO
- [ ] Bloom
- [ ] TAA 
- [ ] DOF

## Demo-only

- [x] Camera
    - [x] Orbit controls 
    - [x] Orthographic
    - [x] Perspective
    - [x] Initial fit for AABB (not perfect, but good enough) 
