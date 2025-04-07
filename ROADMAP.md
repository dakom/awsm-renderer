high-level, keep going through gltf models, one at a time, making them each work
starting with minimal and feature-tests

# Next up

## Sparse accessors

Might actually be working fine, values are logging correctly, but without camera it's hard to see

## Camera

finish up camera stuff (is currently updating buffer, need to propogate to shader, bind groups, etc.)
semi-related to scene graph stuff, but not really, can have minimal "transform" multiplication without a real model matrix

## non-auto pipeline

need to do this... also, use a key and lookup so we get the real object, not just same values

## Scene graph

* hierarchy with slotmap stuff
* transforms everywhere
