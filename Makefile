help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: keys wasm abi circuits state allmacros contracts ## Build everything

abi: ## Build the ABI
	$(MAKE) -C ./rusk-abi all
	
allmacros: ## Build the workspace macro libs and test them
	$(MAKE) -C ./macros all

keys: ## Create the keys for the circuits
	$(MAKE) -C ./rusk-recovery keys

state: wasm ## Create the network state
	$(MAKE) -C ./rusk-recovery state

wasm: ## Generate the WASM for all the contracts
	$(MAKE) -C ./contracts $@
	$(MAKE) -C ./test-utils $@
	$(MAKE) -C ./rusk-abi $@

circuits: ## Build circuit crates
	$(MAKE) -j -C ./circuits all

contracts: ## Execute the test for all contracts
	$(MAKE) -j1 -C ./contracts all

test: keys wasm ## Run the tests
	$(MAKE) -C ./rusk-abi/ $@
	$(MAKE) -j -C ./circuits $@
	$(MAKE) state
	$(MAKE) -C ./macros $@
	$(MAKE) -j1 -C ./contracts $@
	$(MAKE) -C ./rusk/ $@
	$(MAKE) -C ./test-utils $@

run: keys state ## Run the server
	cargo run --release --bin rusk

node: rusk ## Build node binary
	$(MAKE) -C ./node binary

.PHONY: all abi keys state wasm circuits contracts test run help node
