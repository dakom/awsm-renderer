# Development Guide

## Getting Started

It's really as simple as:

```bash
just frontend-dev
```

However, that assumes you have all the prerequisites installed and media assets downloaded.


## Prerequisites 

There are a few prerequisites to set up the development environment.

### Trunk

This is used to build and run the frontend demo. Kinda hard to develop if you can't see the output :)

See https://trunkrs.dev/ for more info 

### Justfile

Used to simplify common tasks like building and running the demo

See https://github.com/casey/just for more info

### Cmgen

cmgen from filament is used to create environment maps and IBL textures

1. clone https://github.com/google/filament
2. build and install: `./build.sh -i release` (make sure cmake, ninja, xcode, etc. are installed)
3. add path to global path: `export PATH="path/to/filament-repo/out/release/filament/bin:$PATH"`

### KTX tools

Used to re-package into ktx2 containers

Use the releases: https://github.com/KhronosGroup/KTX-Software/releases

## Project layout

* [awsm-renderer](crates/renderer): The renderer in all its glory 
* [awsm-renderer-core](crates/renderer-core): Wraps the WebGPU API with very little opinion, just a nicer Rust API
* [frontend](crates/frontend): Just for demo and debugging purposes 
* [docs](docs): Documentation
* [media](media): Media assets for the demo scenes
* [licenses](licenses): Any third-party licenses needed for demo purposes

## Media

For the sake of keeping the repo clean, media files are referenced remotely on the release build, and can be downloaded locally to gitignored directories for dev builds.

Currently, these need to be manually cloned/downloaded (not via git submodules). Clone into `media` the following repos:

1. https://github.com/KhronosGroup/glTF-Sample-Assets.git
2. https://github.com/dakom/awsm-renderer-assets.git
