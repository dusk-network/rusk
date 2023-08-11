help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run Rusk tests
	@cargo test \
		-vvv \
		--release \
		-- --nocapture
			
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings

.PHONY: test help clippy 
