help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

wasm: ## Build the WASM files
	@cargo rustc \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

check: wasm ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

dummy-test-contracts: ## Create a set of dummy contracts to be used in the tests
	$(MAKE) -C tests wasm

test: wasm dummy-test-contracts ## Perform the contract tests defined in the host module
	@cargo test --release
		

.PHONY: wasm check test help
