help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

keys: ## Build circuit keys
	cargo build --release -vv

test: ## Run Rusk tests
	@cargo test \
		--release \
		-vv \
		-- --nocapture

.PHONY: keys test help
