# awsm-renderer

[Live Demo](https://dakom.github.io/awsm-renderer)

# OVERVIEW

**`awsmrenderer`** is a browser-based Rust/WASM/WebGPU renderer

It's specifically for the browser in that it does *not* use wgpu, but rather uses the WebGPU API directly via the web-sys bindings. While this is somewhat unconventional in the Rust ecosystem, it allows for a more direct mapping to the WebGPU API and potentially better performance, control, and easier debugging.

That said, it's not unthinkable for a future version to support native binaries as well ;)

# ARCHITECTURE

This renderer uses a **visibility buffer+ hybrid** approach, enabling efficient shading without the drawbacks of classic deferred or forward+ rendering.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for insight into how this all works and where the tradeoffs are made. 

# STATUS

Nothing much to see here yet, early days, slow-moving hobby and learning in progress :)

See [ROADMAP](docs/ROADMAP.md) for details.

# DEVELOPMENT

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for details on setting up the development environment, building, and running the examples.

# NON-GOALS

### ECS (or any other game framework)

This is a renderer, not a full game engine or framework. There is no entity-component-system (ECS) or any other opinionated way to organize game objects.

However, there is a transform-based scene graph, and all the data structures are designed to be very easy and efficient to manipulate and integrate with an ECS or other game framework by way of "keys" (TransformKey, MeshKey, MaterialKey, etc.)

Feel free to think of these keys as components and assign them to some EntityId of your choice.

It's _very fast_ to update any data in the system many times per-tick (e.g. for physics), it's almost just a memcpy, and the data is only uploaded to the GPU once per tick if needed. Ultimately you can have a very dynamic scene without worrying about overhead (especially when just transforming objects - inserting/removing may hit some very small burps when bind groups need to be recreated. See [ARCHITECTURE](docs/ARCHITECTURE.md) for more details)

### Physics

The renderer does include transformation, morphs, skins, and animation support, but does not include any physics engine or collision detection.

It's expected that another subsystem using this renderer would handle physics/collision detection separately, and provide the resulting transforms/animations to the renderer.

### Material system

There is no easy-to-work-with material system, shader graph, or node-based editor. Instead, materials are hardcoded structures with a specific memory alignment and size, with code on both the Rust and WGSL side that must be in sync. 

However, the foundation is there to allow building a higher-level abstraction, and material tooling could be built on top of this renderer.

### Game world culling

This really depends on the specific needs of a project. Some examples:

* no culling at all (e.g. a fighting game)
* portal-based (e.g. a first-person shooter in an interior)
* space partitioning (e.g. in an open world game).
* quadtrees (e.g. in top-down view)

However, due to the visibility buffer optimization, the impact of rendering unnecessary geometry does not reach the shading stage. Also, frustum culling will eliminate other game world objects... so the only optimization would really be to reduce the frustum culling tests which are already very cheap.


# GRAVEYARD

I've taken some stabs at some variation of this sorta thing before, got a few battle scars along the way. Some projects got further than others:

* [Pure3d (typescript + webgl1)](https://github.com/dakom/pure3d-typescript)
* [Shipyard ECS (webgl2)](https://github.com/dakom/shipyard-webgl-renderer)
* [WebGL1+2 Rust bindings](https://github.com/dakom/awsm-web/tree/master/crate/src/webgl)
