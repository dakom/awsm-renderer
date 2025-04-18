# [live demo](https://dakom.github.io/awsm-renderer)

# WHAT IT HOPES TO BE 

A browser-based Rust/WASM/WebGPU renderer, without using a full game engine framework like Bevy or a modular ECS like Shipyard (bring your own game engine!).

This does *not* use wgpu, but rather uses the WebGPU API directly via the `web-sys` bindings. This is a bit of a departure from the Rust ecosystem, but it allows for a more direct mapping to the WebGPU API and potentially better performance, control, and easier debugging.

# STATUS

Nothing much to see here yet, early days, slow-moving hobby and learning in progress :)

See [ROADMAP](ROADMAP.md) for details.

# CRATES

* [awsm-renderer](crates/renderer): The renderer in all its glory 
* [awsm-renderer-core](crates/renderer-core): Wraps the WebGPU API with very little opinion, just a nicer Rust API
* [frontend](crates/frontend): Just for demo and debugging purposes 

# MEDIA

For the sake of keeping the repo clean, media files are referenced remotely on the release build, and be downloaded locally to gitignored directories for dev builds. 

See [media/README.md](media/README.md) for more details.

# GRAVEYARD

I've taken some stabs at some variation of this sorta thing before. Some projects got further than others:

* [Pure3d (typescript + webgl1)](https://github.com/dakom/pure3d-typescript)
* [Shipyard ECS (webgl2)](https://github.com/dakom/shipyard-webgl-renderer)
* [WebGL1+2 Rust bindings](https://github.com/dakom/awsm-web/tree/master/crate/src/webgl)
