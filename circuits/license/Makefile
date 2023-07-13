help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: ## Build the circuits
	@cargo build --release \
		--features builder

test: ## Run the transfer circuits tests
	@cargo test --release \
		--features builder

clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

.PHONY: all test help
