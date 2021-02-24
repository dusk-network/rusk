CONTRACTS := $(wildcard ./contracts/*/.)
CIRCUITS := $(wildcard ./circuits/*/.)

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

abi: ## Build the ABI and test it
	$(MAKE) -C ./rusk-abi test

keys: ## Create the keys for the circuits
	$(MAKE) -C ./rusk keys

wasm: ## Generate the WASM for all the contracts
	for dir in $(CONTRACTS); do \
        $(MAKE) -C $$dir $@ ; \
    done

circuits: ## Build and test circuit crates
	for dir in $(CIRCUITS); do \
        $(MAKE) -C $$dir test ; \
    done

contracts: wasm ## Execute the test for all contracts
	for dir in $(CONTRACTS); do \
        $(MAKE) -C $$dir test ; \
    done

test: abi wasm circuits contracts ## Run the tests
	$(MAKE) -C ./rusk/ $@
	
run: wasm ## Run the server
	cargo run --release

.PHONY: abi keys wasm circuits contracts test run help
