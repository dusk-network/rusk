all: ## Build the transfer contract
	@cargo build --release

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

check: ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

test: ## Perform the contract tests defined in the host module
	@cargo test --release

wasm: ## Build the data-driver WASM files
	@wasm-pack build --target nodejs --release
			
clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy --release --target wasm32-unknown-unknown -- -D warnings
	
doc: ## Run doc gen
	@cargo doc --release

.PHONY: all check test wasm help
