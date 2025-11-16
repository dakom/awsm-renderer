# Next up

- Rewrite entire pipeline to match README doc (Visiblity+)
  - See [TODO.md](TODO.md)

- get demo working again
  - fix "dev-release" mode (just base path?)

- support KHR_texture_transform
  - see base color comparison for example

- load lights via KHR_lights_punctual
  - point, spot, directional

- Support different kinds of Materials
  - should just be a simple gate on the material meta, this is the beauty of the compute shader driven approach
  - unlit as example?

- at this point can probably use for demos :)

- Get started with light culling pass
  - research best practices
  - should optimize for opaque pass (i.e. only light fragments that made it to the screen?)

- optimize wgsl structs
  - use FooPacked (or FooRaw) and members should be vec4
  - e.g. MeshMeta, Lights, etc. (some may already be done)

- get rid of 256 byte alignment for mesh meta?
  - maybe only necessary for uniforms?

- make it easier to configure initial sizes for dynamic buffers
  - derive from scanning gltf?

# Multithreading

Dynamic/Uniform storages could be SharedArrayBuffer
Requires more design/thought (don't want to expose raw manipulation)

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
    - [x] PBR metallic-roughness
        - [x] base color
        - [x] metallic
        - [x] roughness
        - [x] normal
        - [x] occlusion
        - [x] emissive
    - [x] mipmaps
- Lighting
    - [x] IBL
        - [x] diffuse irradiance
        - [x] specular prefiltered
        - [x] BRDF LUT
    - [x] punctual lights


## Drawing
- [x] non-indexed
- [x] indexed
- [x] instancing
- [ ] Early z pre-pass
- [x] Opaque front to back
- [x] Transparent back to front

## Textures
- [x] 2D textures
- [x] Mipmaps (port https://github.com/JolifantoBambla/webgpu-spd)

## Skybox
- [x] load ktf
- [x] generate colors
- [x] generate pseudo-sky

## IBL Helpers
- [x] Load ktf
- [x] generate colors
- [x] Document third-party tooling and easy flow

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
- [x] TAA
- [ ] DOF

## Demo-only

- [x] Camera
    - [x] Orbit controls
    - [x] Orthographic
    - [x] Perspective
    - [x] Initial fit for AABB (not perfect, but good enough)

## Optimizations

- [ ] Multithreading
- [x] MegaTextures
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
- [x] AABB
    - Only update when transform changes
- [x] Morphs and Skins
    - Global buffers and dirty flags
- [x] Meshes
    - One vertex buffer
    - One index buffer
    - Gpu gating on dirty flag
- [x] Transform instancing
    - One dynamic uniform bind group for transforms
    - Gpu gating on dirty flag
- Camera
    - [x] Single uniform buffer
    - [ ] Frustum culling
