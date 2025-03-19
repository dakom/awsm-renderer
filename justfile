# Frontend
FRONTEND_DEV_PORT := "9080"
FRONTEND_MEDIA_DEV_PORT := "9082"
FRONTEND_OUT_DIR := ".build-artifacts/frontend"

frontend-dev:
    #!/bin/bash -eux
    just frontend-dev-localmedia &
    just inner_frontend-dev &
    trap 'kill $(jobs -pr)' EXIT
    wait

inner_frontend-dev:
	@cd crates/frontend && trunk serve --port {{FRONTEND_DEV_PORT}} --watch . --watch "../../../media"

frontend-dev-localmedia:
    @cd media && http-server --gzip --cors -p {{FRONTEND_MEDIA_DEV_PORT}} 

# BUILD

frontend-build:
    @rm -rf "{{FRONTEND_OUT_DIR}}"
    @mkdir -p "{{FRONTEND_OUT_DIR}}"
    @just inner_frontend-build
    @cp -r "crates/frontend/dist/." "{{FRONTEND_OUT_DIR}}"
    @cp -r "media" "{{FRONTEND_OUT_DIR}}/media"
    @cp "{{FRONTEND_OUT_DIR}}/index.html" "{{FRONTEND_OUT_DIR}}/404.html"

inner_frontend-build:
    @cd crates/frontend && trunk build --release --public-url=https://dakom.github.io/awsm-renderer/