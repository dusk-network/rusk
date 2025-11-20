all: wasm clippy

wasm: ## Generate the optimized WASM for the contract given
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
    	cargo build \
    		--release \
    		--color=always \
			-Z build-std=core,alloc \
    		--target wasm32-unknown-unknown

test:

clippy:
	@cargo clippy -Z build-std=core,alloc --release --target wasm32-unknown-unknown -- -D warnings

doc:

.PHONY: all test wasm
