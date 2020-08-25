help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

wasm: ## Generate the WASM for the contract given (e.g. make wasm for=transfer)
	@cargo rustc \
		--manifest-path=contracts/$(for)/Cargo.toml \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

contracts: ## Generate the WASM for all the contracts
		@for file in `find contracts -name "Cargo.toml"` ; do \
			cargo rustc \
				--manifest-path=$${file} \
				--release \
				--target wasm32-unknown-unknown \
				-- -C link-args=-s; \
		done

test: ## Run the tests
		@make contracts && \
			cargo test --release --features "tests_travis" -- --nocapture 

run: ## Run the server
		@make contracts && \
			cargo run --release

.PHONY: help wasm contracts test run
