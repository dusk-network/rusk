help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

check: wasm ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

test: utils wasm ## Perform the contract tests defined in the host module
	@cargo test --release

utils: ## Build the WASM files of the test utils
	$(MAKE) -C ../../ $@

wasm: ## Build the WASM files
	@cargo rustc \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

.PHONY: check test utils wasm help
