wasm: ## Generate the optimized WASM for the contract given
	@cargo rustc \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s
