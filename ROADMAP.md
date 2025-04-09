high-level, keep going through gltf models, one at a time, making them each work
starting with minimal and feature-tests

# Next up

## Transforms
Use mapped range instead of writeBuffer()

If buffer is relatively small or there's many changes, just a single copy_from_slice is fine
If buffer is large and only few changes, multiple copy_from_slice calls are better

Maybe determination should be a percentage, i.e. if <10% of the buffer is changed, use chunks, otherwise use a single copy

## Gltf models

#### Simple Morph
bones, animation, and stuff :)

## Camera

* Arc ball controls