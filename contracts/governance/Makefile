help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: ## Build the governance contract
	@cargo build --release

check: wasm ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

test: wasm ## Perform the contract tests defined in the host module
	@cargo test --release

wasm: ## Generate the optimized WASM for the contract given
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
    	cargo build \
    		--release \
    		--color=always \
    		-Z build-std=core,alloc,panic_abort \
    		-Z build-std-features=panic_immediate_abort \
    		--target wasm32-unknown-unknown

.PHONY: all check test wasm help
