TARGET_DIR:="../../target/dusk"

all: wasm

wasm: ## Generate the optimized WASM for the contract given
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
	CARGO_TARGET_DIR=$(TARGET_DIR) \
    	cargo +dusk build \
    		--release \
    		--color=always \
    		-Z build-std=core,alloc,panic_abort \
    		-Z build-std-features=panic_immediate_abort \
    		--target wasm32-unknown-unknown

test:

clippy: 

doc:

.PHONY: all test wasm
