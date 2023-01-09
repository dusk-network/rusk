help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

abi: ## Build the ABI and test it
	$(MAKE) -C ./rusk-abi test

allmacros: ## Build the workspace macro libs and test them
	$(MAKE) -C ./macros test

keys: ## Create the keys for the circuits
	$(MAKE) -C ./rusk-recovery keys

state: wasm ## Create the network state
	$(MAKE) -C ./rusk-recovery state

wasm: ## Generate the WASM for all the contracts
	$(MAKE) -C ./contracts $@
	$(MAKE) -C ./test-utils $@
	$(MAKE) -C ./rusk-abi $@

circuits: ## Build and test circuit crates
	$(MAKE) -j -C ./circuits test

contracts: ## Execute the test for all contracts
	$(MAKE) -j1 -C ./contracts test

test: keys wasm abi circuits state allmacros contracts ## Run the tests
	$(MAKE) -C ./rusk/ $@
	$(MAKE) -C ./test-utils $@

run: keys state ## Run the server
	cargo run --release --bin rusk

node: rusk ## Build node binary
	$(MAKE) -C ./node binary

.PHONY: abi keys state wasm circuits contracts test run help node
