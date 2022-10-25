wasm: ## Generate the optimized WASM for the contract given
	@RUSTFLAGS="-C link-args=-zstack-size=65536" \
    	cargo build \
    	  --release \
    	  --color=always \
    	  -Z build-std=core,alloc,panic_abort \
    	  -Z build-std-features=panic_immediate_abort \
    	  --target wasm32-unknown-unknown
