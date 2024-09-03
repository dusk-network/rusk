TARGET_DIR:="../../target/dusk"

all: ## Build the ABI
	cargo build

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test:
	cargo test --release


wasm: ## Build the WASM files
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
	CARGO_TARGET_DIR=$(TARGET_DIR) \
	cargo +dusk build \
		--release \
		--no-default-features \
		--color=always \
		-Z build-std=core,alloc \
		--target wasm64-unknown-unknown

clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

.PHONY: all help test
