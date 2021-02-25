help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

abi: ## Build the ABI and test it
	$(MAKE) -C ./rusk-abi test

keys: ## Create the keys for the circuits
	$(MAKE) -C ./rusk keys

wasm: ## Generate the WASM for all the contracts
	$(MAKE) -C ./contracts wasm

circuits: ## Build and test circuit crates
	$(MAKE) -C ./circuits test

contracts: ## Execute the test for all contracts
	$(MAKE) -C ./contracts test

test: abi circuits contracts ## Run the tests
	$(MAKE) -C ./rusk/ $@
	
run: wasm ## Run the server
	cargo run --release

.PHONY: abi keys wasm circuits contracts test run help
