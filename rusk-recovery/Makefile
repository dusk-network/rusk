help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run tests
	@RUST_LOG=error cargo test \
		--release \
		--all-features \
		-- --test-threads 1

test-debug: ## Run tests with full output
	@cargo test \
		--release \
		--all-features \
		-- --nocapture \
		--test-threads 1
			
clippy: ## Run clippy
	@cargo clippy --release --all-features -- -D warnings
	@cargo clippy --release --no-default-features -- -D warnings
	@cargo clippy --release --no-default-features --features state -- -D warnings
	@cargo clippy --release --no-default-features --features keys -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

.PHONY: test test-debug clippy help
