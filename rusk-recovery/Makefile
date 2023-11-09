help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run tests
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

.PHONY: test clippy help
