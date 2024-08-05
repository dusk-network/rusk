all: ## Build the ABI
	cargo build

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test:
	cargo test --release --features zk 
	cargo test --release --no-run
			
clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings

.PHONY: all help test
