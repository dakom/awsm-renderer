# Frontend
FRONTEND_DEV_PORT := "9080"
FRONTEND_GLTF_SAMPLES_DEV_PORT := "9082"
FRONTEND_ADDITIONAL_ASSETS_DEV_PORT := "9083"
FRONTEND_OUT_DIR := ".build-artifacts/frontend"

frontend-dev:
    #!/bin/bash -eu
    just frontend-dev-localmedia-gltf-samples &
    just frontend-dev-localmedia-additional-assets &
    just inner_frontend-dev false &
    trap 'kill $(jobs -pr)' EXIT
    wait

@frontend-dev-release:
    #!/bin/bash -eu
    just frontend-dev-localmedia-gltf-samples &
    just frontend-dev-localmedia-additional-assets &
    just inner_frontend-dev true &
    trap 'kill $(jobs -pr)' EXIT
    wait

@inner_frontend-dev RELEASE:
	cd crates/frontend && trunk serve --port {{FRONTEND_DEV_PORT}} --release {{RELEASE}} --watch . --watch "../../../media" --watch ../renderer --watch ../renderer-core

@frontend-dev-localmedia-gltf-samples:
    cd media && http-server --gzip --cors -p {{FRONTEND_GLTF_SAMPLES_DEV_PORT}}

@frontend-dev-localmedia-additional-assets:
    cd ../awsm-renderer-assets && http-server --gzip --cors -p {{FRONTEND_ADDITIONAL_ASSETS_DEV_PORT}}

# BUILD

@frontend-build:
    rm -rf "{{FRONTEND_OUT_DIR}}"
    mkdir -p "{{FRONTEND_OUT_DIR}}"
    just inner_frontend-build
    cp -r "crates/frontend/dist/." "{{FRONTEND_OUT_DIR}}"
    cp -r "media" "{{FRONTEND_OUT_DIR}}/media"
    cp "{{FRONTEND_OUT_DIR}}/index.html" "{{FRONTEND_OUT_DIR}}/404.html"

@inner_frontend-build:
    cd crates/frontend && trunk build --release --public-url=https://dakom.github.io/awsm-renderer/
