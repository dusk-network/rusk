help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: ## Build the circuits
	@cargo build --release

test: ## Run the transfer circuits tests
	@cargo test --release

clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

.PHONY: all test help
