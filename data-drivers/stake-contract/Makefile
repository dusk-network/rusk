CRATE_NAME_DASHED := $(shell cargo metadata --no-deps --format-version 1 --manifest-path Cargo.toml | jq -r '.packages[] | select(.manifest_path == "$(realpath Cargo.toml)") | .name')
CRATE_NAME := $(subst -,_,$(CRATE_NAME_DASHED))

# Detect workspace root so we know where target/ actually lives
WORKSPACE_ROOT := $(shell cargo metadata --no-deps --format-version 1 --manifest-path Cargo.toml | jq -r '.workspace_root')

WASM_OPT ?= $(shell command -v wasm-opt 2>/dev/null)
WASM_TARGET = wasm32-unknown-unknown
WASM_RELEASE_DIR = $(WORKSPACE_ROOT)/target/$(WASM_TARGET)/release
WASM_FILE = $(WASM_RELEASE_DIR)/$(CRATE_NAME).wasm
WASM_OPT_FILE = $(WASM_RELEASE_DIR)/$(CRATE_NAME)_opt.wasm


all: ## Build the transfer contract
	@cargo build --release

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

check: ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

test: ## Perform the contract tests defined in the host module
	@cargo test --release


wasm-pack: ## Build the data-driver WASM files with wasm-pack
	@if [ -z "$(WASM_OPT)" ]; then \
		echo "wasm-opt not found. Installing via cargo-binstall with system toolchain..."; \
		cargo +stable install -f wasm-opt; \
		WASM_OPT=$$(command -v wasm-opt); \
	fi
	@echo "Building WASM for crate: $(CRATE_NAME)"
	@cargo build --release --features ffi,js --target $(WASM_TARGET) --manifest-path Cargo.toml
	@echo "Optimizing with wasm-opt..."
	@wasm-opt -Oz --strip-debug -o $(WASM_OPT_FILE) $(WASM_FILE)
	@echo "WASM build complete: $(WASM_OPT_FILE)"

wasm: ## Build the data-driver WASM files
	@if [ -z "$(WASM_OPT)" ]; then \
		echo "wasm-opt not found. Installing via cargo-binstall with system toolchain..."; \
		cargo +stable install -f wasm-opt; \
		WASM_OPT=$$(command -v wasm-opt); \
	fi
	@echo "Building WASM for crate: $(CRATE_NAME)"
	@cargo build --release --features ffi --target $(WASM_TARGET) --manifest-path Cargo.toml
	@echo "Optimizing with wasm-opt..."
	@wasm-opt -Oz --strip-debug -o $(WASM_OPT_FILE) $(WASM_FILE)
	@echo "WASM build complete: $(WASM_OPT_FILE)"
			
clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy --release --target wasm32-unknown-unknown --features wasm-bindgen -- -D warnings
	@cargo clippy --release --target wasm32-unknown-unknown --features ffi -- -D warnings
	
doc: ## Run doc gen
	@cargo doc --release

.PHONY: all check test wasm help
