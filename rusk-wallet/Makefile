help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Builds both native and WASM versions of the code
	cargo b --release --target=wasm32-unknown-unknown
	cargo b

test: build ## Run wallet tests
	cargo test

.PHONY: build test help
