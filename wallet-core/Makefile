all: ## Build the ABI
	cargo build

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test:
	cargo test --release

wasm: ## Build the WASM files
	@cargo wasm

clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

headers: ## Generate C headers
	@cbindgen --crate dusk-wallet-core --output wallet-core.h --config cbindgen.toml

.PHONY: all help test
