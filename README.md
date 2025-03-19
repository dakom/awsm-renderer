# WHAT IT HOPES TO BE 

A relatively pure Rust/WASM/WebGPU renderer, without using a full game engine framework like Bevy or a modular ECS like Shipyard.

Still tinkering around, but the current idea is to *not* use wgpu, but rather use the WebGPU API directly via the `web-sys` bindings. This is a bit of a departure from the Rust ecosystem, but it allows for a more direct mapping to the WebGPU API and potentially better performance, control, and easier debugging.

# STATUS

Nothing to see here yet, early days, slow-moving hobby and learning in progress :)

# GRAVEYARD

I've taken some stabs at some variation of this sorta thing before. Some projects got further than others:

* [Pure3d (typescript + webgl1)](https://github.com/dakom/pure3d-typescript)
* [Shipyard ECS (webgl2)](https://github.com/dakom/shipyard-webgl-renderer)
* [WebGL1+2 Rust bindings](https://github.com/dakom/awsm-web/tree/master/crate/src/webgl)
