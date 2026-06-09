.DEFAULT_GOAL := help

SHELL := /bin/bash

TARGET ?= .
RUN_ARGS ?=
SARIF_FILE ?= owui-lint.sarif
JSON_FILE ?= owui-lint-report.json
BIN ?= owui-lint
IMAGE ?= owui-lint:local
RUST_IMAGE ?= rust:1.93-bookworm
INSTALL_DIR ?= ./bin
NPM_CACHE ?= .npm-cache
ASTRO_TELEMETRY_DISABLED ?= 1
RUST_DOCKER_RUN = docker run --rm -v "$$(pwd):/work" -w /work $(RUST_IMAGE) bash -lc 'export PATH=/usr/local/cargo/bin:$$PATH && rustup component add rustfmt clippy && make $(1)'

.PHONY: help build build-release release fmt fmt-check lint test test-scripts assets-sync assets-check owui-sources-sync owui-sources-check docs-sync docs-check docs-site-install docs-site-build docs-site-check vscode-extension-install vscode-extension-build vscode-extension-package vscode-extension-check check run run-json run-sarif dist install ci ci-check clean docker-build docker-run docker-install docker-check docker-ci

help: ## Show available commands
	@awk 'BEGIN {FS = ":.*##"; print "Available targets:"} /^[a-zA-Z0-9_.-]+:.*##/ {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build debug binary
	cargo build --locked

build-release: ## Build release binary
	cargo build --release --locked

release: build-release ## Alias for release build

fmt: ## Format Rust code
	cargo fmt

fmt-check: ## Check Rust formatting
	cargo fmt -- --check

lint: ## Run clippy checks
	cargo clippy --locked -- -D warnings

test: ## Run Rust tests
	cargo test --locked

test-scripts: ## Run shell script tests
	bash scripts/test-new-rule.sh
	bash scripts/test-sync-readme.sh

assets-sync: ## Mirror master assets/ icons into docs and editor copies
	bash scripts/sync-assets.sh --write

assets-check: ## Verify generated asset copies match the assets/ masters
	bash scripts/sync-assets.sh --check

owui-sources-sync: ## Fetch upstream Open WebUI source/docs (ground truth for OWSEC rules)
	bash scripts/sync-owui-sources.sh --write

owui-sources-check: ## Fail if bundled OWUI ground-truth copies drifted from upstream
	bash scripts/sync-owui-sources.sh --check

docs-sync: ## Regenerate README and generated docs site references from live CLI output
	cargo run --locked --bin docs-sync -- --write

docs-site-install: ## Install docs site npm dependencies
	npm ci --prefix docs --cache $(NPM_CACHE)

docs-site-build: docs-site-install ## Build the Starlight docs site
	ASTRO_TELEMETRY_DISABLED=$(ASTRO_TELEMETRY_DISABLED) npm run build --prefix docs --cache $(NPM_CACHE)

docs-site-check: docs-site-install ## Audit and validate the Starlight docs site
	npm audit --prefix docs --cache $(NPM_CACHE)
	ASTRO_TELEMETRY_DISABLED=$(ASTRO_TELEMETRY_DISABLED) npm run check --prefix docs --cache $(NPM_CACHE)

docs-check: assets-check ## Fail if generated docs/assets are out of date and docs site is valid
	cargo run --locked --bin docs-sync -- --check
	$(MAKE) docs-site-check

vscode-extension-install: ## Install VS Code extension npm dependencies
	npm ci --prefix editors/vscode --cache $(NPM_CACHE)

vscode-extension-build: vscode-extension-install ## Type-check and bundle the VS Code extension
	npm run compile --prefix editors/vscode --cache $(NPM_CACHE)

vscode-extension-package: vscode-extension-install ## Build the VS Code extension VSIX package
	npm run vsix --prefix editors/vscode --cache $(NPM_CACHE)

vscode-extension-check: vscode-extension-package ## Verify the VS Code extension can be packaged

check: fmt-check lint test test-scripts assets-check ## Run all quality gates

run: ## Run owui-lint against TARGET (default: .)
	cargo run --locked -- $(TARGET) $(RUN_ARGS)

run-json: RUN_ARGS += --format json --output $(JSON_FILE)
run-json: run ## Run owui-lint and write JSON report (TARGET, JSON_FILE)

run-sarif: RUN_ARGS += --format sarif --output $(SARIF_FILE)
run-sarif: run ## Run owui-lint and write SARIF report (TARGET, SARIF_FILE)

dist: release ## Build distributable release binary

install: ## Install binary from this workspace
	cargo install --path . --locked --force

ci: ci-check run-sarif ## CI pipeline: checks + SARIF generation

ci-check: check ## CI check job parity (format, clippy, tests, script tests)

clean: ## Remove build, cache, and report artifacts
	rm -rf target
	rm -f $(SARIF_FILE) $(JSON_FILE)

docker-build: ## Build Docker image (no local Rust/Cargo required)
	docker build -t $(IMAGE) .

docker-run: ## Run linter via Docker image against TARGET (mounts current repo)
	docker run --rm -v "$$(pwd):/work" -w /work $(IMAGE) $(TARGET)

docker-install: docker-build ## Extract binary from Docker image into INSTALL_DIR
	@mkdir -p "$(INSTALL_DIR)"
	@cid=$$(docker create $(IMAGE)); \
	docker cp $$cid:/usr/local/bin/owui-lint "$(INSTALL_DIR)/$(BIN)"; \
	docker rm $$cid >/dev/null; \
	chmod +x "$(INSTALL_DIR)/$(BIN)"; \
	echo "Installed $(INSTALL_DIR)/$(BIN)"

docker-check: ## Run check target inside official Rust Docker image
	$(call RUST_DOCKER_RUN,ci-check)

docker-ci: ## Run ci target (checks + SARIF) inside official Rust Docker image
	$(call RUST_DOCKER_RUN,ci)
