all: keys wasm abi allcircuits state contracts rusk web-wallet ## Build everything

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

abi: ## Build the ABI
	$(MAKE) -C ./rusk-abi all

keys: ## Create the keys for the circuits
	$(MAKE) -C ./rusk recovery-keys

state: keys wasm ## Create the network state
	$(MAKE) -C ./rusk recovery-state

wasm: setup-compiler ## Generate the WASM for all the contracts and wallet-core
	$(MAKE) -C ./contracts $@
	$(MAKE) -C ./wallet-core $@

allcircuits: ## Build circuit crates
	$(MAKE) -j -C ./circuits all

contracts: ## Execute the test for all contracts
	$(MAKE) -j1 -C ./contracts all

test: keys wasm ## Run the tests
	$(MAKE) -C ./rusk-abi/ $@
	$(MAKE) -C ./execution-core/ $@
	$(MAKE) -j -C ./circuits $@
	$(MAKE) state
	$(MAKE) -j1 -C ./contracts $@
	$(MAKE) -C ./rusk-recovery $@
	$(MAKE) -C ./rusk-prover/ $@
	$(MAKE) -C ./node-data $@
	$(MAKE) -C ./consensus $@
	$(MAKE) -C ./node $@
	$(MAKE) -C ./wallet-core $@
	$(MAKE) -C ./rusk/ $@
			
clippy: ## Run clippy
	$(MAKE) -C ./execution-core/ $@
	$(MAKE) -j -C ./circuits $@
	$(MAKE) -j1 -C ./contracts $@
	$(MAKE) -C ./rusk-abi $@
	$(MAKE) -C ./rusk-profile $@
	$(MAKE) -C ./rusk-recovery $@
	$(MAKE) -C ./rusk-prover/ $@
	$(MAKE) -C ./node-data $@
	$(MAKE) -C ./consensus $@
	$(MAKE) -C ./node $@
	$(MAKE) -C ./wallet-core $@
	$(MAKE) -C ./rusk/ $@

doc: ## Run doc gen
	$(MAKE) -C ./execution-core/ $@
	$(MAKE) -j -C ./circuits $@
	$(MAKE) -C ./consensus $@
	$(MAKE) -j1 -C ./contracts $@
	$(MAKE) -C ./node $@
	$(MAKE) -C ./node-data $@
	$(MAKE) -C ./rusk/ $@
	$(MAKE) -C ./rusk-abi $@
	$(MAKE) -C ./rusk-profile $@
	$(MAKE) -C ./rusk-prover/ $@
	$(MAKE) -C ./rusk-recovery $@
	$(MAKE) -C ./wallet-core/ $@

bench: keys wasm  ## Bench Rusk
	$(MAKE) -C ./rusk bench

run: keys state web-wallet ## Run the server
	$(MAKE) -C ./rusk/ $@

rusk: keys state web-wallet ## Build rusk binary
	$(MAKE) -C ./rusk build

web-wallet: ## build the static files of the web wallet
	$(MAKE) -C ./web-wallet all 

COMPILER_VERSION=v0.2.0
setup-compiler: ## Setup the Dusk Contract Compiler
	@./scripts/setup-compiler.sh $(COMPILER_VERSION)

.PHONY: all abi keys state wasm allcircuits contracts test bench run help rusk web-wallet setup-compiler
