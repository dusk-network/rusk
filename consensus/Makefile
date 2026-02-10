help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

release:
	cargo clippy; cargo b --release

test: ## Run tests
	@cargo test --release -- --nocapture
	
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

clean:
	@cargo clean

.PHONY: test help clean release
