help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run tests
	@RUST_LOG=error cargo test \
		--release \
		-- --test-threads 8

test-debug: ## Run tests with full output
	@cargo test \
		--release \
		-- --test-threads 8 \
		--nocapture
			
clippy: ## Run clippy
	@cargo clippy --no-default-features --release -- -D warnings
	@cargo clippy \
		--no-default-features \
		--features archive \
		--release \
		-- -D warnings

doc: ## Run doc gen
	@cargo doc --release

build: ## Build rusk binary
	@cargo build --release

.PHONY: help test test-debug clippy doc build
