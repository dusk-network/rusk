all: ## Generate the optimized WASM for the contract given
	@cargo rustc \
		--manifest-path=./Cargo.toml \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s
