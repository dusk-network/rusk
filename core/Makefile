all: ## Build the ABI
	cargo build

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test:
	cargo test --release --features zk,serde
	cargo test --release --no-run
			
clippy: ## Run clippy
	@cargo clippy --release --features serde -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

.PHONY: all help test
