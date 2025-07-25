TARGET_DIR:="../../target/dusk"

all: ## Build the transfer contract
	@cargo build --release

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

check: wasm ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

test: wasm ## Perform the contract tests defined in the host module
	@cargo test --release

wasm: ## Build the WASM files
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
	CARGO_TARGET_DIR=$(TARGET_DIR) \
	cargo +dusk build \
		--release \
		--color=always \
		-Z build-std=core,alloc \
		--target wasm64-unknown-unknown
			
clippy: ## Run clippy
	@cargo +dusk clippy --all-features --release -- -D warnings
	@cargo +dusk clippy -Z build-std=core,alloc --release --target wasm32-unknown-unknown -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

.PHONY: all check test wasm help
