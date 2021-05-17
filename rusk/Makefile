help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

keys: clean_uds ## Build circuit keys
	cargo build --release

test: clean_uds ## Run Rusk tests
	@cargo test \
		--release \
		-- --nocapture

clean_uds: ## Removes the /tmp/rusk_listener
	rm -f /tmp/rusk_listener~

.PHONY: keys test clean_uds help
