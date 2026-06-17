all: ## Build the ABI
	cargo build

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test:
	cargo test --release


wasm: ## Build the WASM files
	@bash ../install-nightly-rusk.sh
	@cargo +nightly-rusk wasm

clippy: ## Run clippy
	@bash ../install-nightly-rusk.sh
	@cargo clippy --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings
	@cargo +nightly-rusk wasm-clippy


doc: ## Run doc gen
	@cargo doc --release

.PHONY: all help test
