Need to flesh this out a lot more... right now just loose notes


# Bind groups
Various sources of data can affect multiple bind groups, and are also typically not themselves concerned with the bind groups they affect.

Therefore, when data changes, it marks its change in a global dirty list, and then the bind groups are actually updated right before render pass

Typically this is happens due to buffer resizing, which in webgpu means it creates a new buffer, and the bind group would be pointing to the old buffer

# Textures

Textures are organized into atlases, which are then organized into a megatexture

The megatexture is, conceptually, a single large texture that contains all the smaller textures used in the scene. Due to hardware limitations, it is actually implemented as an array of 2D textures, and these are only uploaded to the GPU as separate texture arrays since there is a limit on maximum depth.

Once the megatexture is composed, `finalize_gpu_textures()` is called to upload the texture arrays to the GPU and rebuild all the pipelines and shaders that need it

# Pipelines / shaders

Some pipelines and shaders cannot be known until until the megatexture is finalized, since they need to know how many texture arrays there are

These are created in `finalize_gpu_textures()`

Renderables check to see if their pipeline has been created in the render loop, and if not, they wait to try again
