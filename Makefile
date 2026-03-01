.DEFAULT_GOAL := help

SHELL := /bin/bash

TARGET ?= .
SARIF_FILE ?= owui-lint.sarif
JSON_FILE ?= owui-lint-report.json
BIN ?= owui-lint
IMAGE ?= owui-lint:local
INSTALL_DIR ?= ./bin

.PHONY: help
help: ## Show available commands
	@awk 'BEGIN {FS = ":.*##"; print "Available targets:"} /^[a-zA-Z0-9_.-]+:.*##/ {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: build
build: ## Build debug binary
	cargo build --locked

.PHONY: build-release
build-release: ## Build release binary
	cargo build --release --locked

.PHONY: release
release: build-release ## Alias for release build

.PHONY: fmt
fmt: ## Format Rust code
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Check Rust formatting
	cargo fmt -- --check

.PHONY: lint
lint: ## Run clippy checks
	cargo clippy --locked -- -D warnings

.PHONY: test
test: ## Run Rust tests
	cargo test --locked

.PHONY: check
check: fmt-check lint test ## Run all quality gates

.PHONY: run
run: ## Run owui-lint against TARGET (default: .)
	cargo run --locked -- $(TARGET)

.PHONY: run-json
run-json: ## Run owui-lint and write JSON report (TARGET, JSON_FILE)
	cargo run --locked -- $(TARGET) --format json --output $(JSON_FILE)

.PHONY: run-sarif
run-sarif: ## Run owui-lint and write SARIF report (TARGET, SARIF_FILE)
	cargo run --locked -- $(TARGET) --format sarif --output $(SARIF_FILE)

.PHONY: dist
dist: build-release ## Build distributable release binary

.PHONY: install
install: ## Install binary from this workspace
	cargo install --path . --locked --force

.PHONY: ci
ci: check run-sarif ## CI pipeline: checks + SARIF generation

.PHONY: clean
clean: ## Remove build, cache, and report artifacts
	rm -rf target
	rm -f $(SARIF_FILE) $(JSON_FILE)

.PHONY: docker-build
docker-build: ## Build Docker image (no local Rust/Cargo required)
	docker build -t $(IMAGE) .

.PHONY: docker-run
docker-run: ## Run linter via Docker image against TARGET (mounts current repo)
	docker run --rm -v "$$(pwd):/work" -w /work $(IMAGE) $(TARGET)

.PHONY: docker-install
docker-install: docker-build ## Extract binary from Docker image into INSTALL_DIR
	@mkdir -p "$(INSTALL_DIR)"
	@cid=$$(docker create $(IMAGE)); \
	docker cp $$cid:/usr/local/bin/owui-lint "$(INSTALL_DIR)/$(BIN)"; \
	docker rm $$cid >/dev/null; \
	chmod +x "$(INSTALL_DIR)/$(BIN)"; \
	echo "Installed $(INSTALL_DIR)/$(BIN)"
