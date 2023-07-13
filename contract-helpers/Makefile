help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run tests
	@cargo test --release -- --nocapture

clean:
	@cargo clean

clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

.PHONY: test help clean
