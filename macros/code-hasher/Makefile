all: ## Build the code-hasher macro
	@cargo build --release 

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: ## Perform the test of the code-hasher macro
	@cargo test \
		--release \
			
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

.PHONY: all test help
