help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

abi: ## Build the ABI and test it
	$(MAKE) -C ./rusk-abi test

macros: ## Build the workspace macro libs and test them
	$(MAKE) -C ./macros test

keys: ## Create the keys for the circuits
	$(MAKE) -C ./rusk keys

wasm: ## Generate the WASM for all the contracts
	$(MAKE) -j -C ./contracts wasm

wallet: ## Build and test the wallet
	$(MAKE) -j -C ./rusk-wallet test

circuits: keys ## Build and test circuit crates
	$(MAKE) -j -C ./circuits test

contracts: ## Execute the test for all contracts
	$(MAKE) -j1 -C ./contracts test

utils: ## Execute the test for utils 
	$(MAKE) -C ./test-utils test

test: abi circuits macros contracts ## Run the tests
	$(MAKE) -C ./rusk/ $@
	$(MAKE) -C ./test-utils test

run: wasm ## Run the server
	cargo run --release

.PHONY: abi keys wasm circuits contracts test run help
