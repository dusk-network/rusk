help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the wallet
	cargo b --release

install: build
	cargo install --path .

clippy: 
	cargo clippy --all-features --release -- -D warnings

test: ## Run wallet tests
	@rm -f /tmp/example.state
	$(MAKE) -C .. prepare-dev
	@../scripts/run-rusk-wallet-tests.sh

.PHONY: build clippy test help 