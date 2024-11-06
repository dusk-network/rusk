all: keys wasm abi allcircuits state contracts rusk rusk-wallet web-wallet ## Build everything

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
	$(MAKE) -C ./rusk-wallet/ $@
			
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
	$(MAKE) -C ./rusk-wallet/ $@

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

bench: keys wasm  ## Bench Rusk & node
	$(MAKE) -C ./node bench
	$(MAKE) -C ./rusk bench

run: keys state web-wallet ## Run the server
	$(MAKE) -C ./rusk/ $@

prepare-dev: keys wasm ## Preparation steps for launching a local node for development
		@cp examples/consensus.keys ~/.dusk/rusk/consensus.keys \
	&& cargo r --release -p rusk -- recovery state --init examples/genesis.toml -o /tmp/example.state || echo "Example genesis state already exists. Not overriding it"

run-dev: ## Launch a local ephemeral node for development
	DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p rusk -- -s /tmp/example.state

run-dev-archive: ## Launch a local ephemeral archive node for development
	DUSK_CONSENSUS_KEYS_PASS=password cargo r --release --features archive -p rusk  -- -s /tmp/example.state

rusk: keys state web-wallet ## Build rusk binary
	$(MAKE) -C ./rusk build

rusk-wallet: ## build the rusk wallet binary
	$(MAKE) -C ./rusk-wallet build 

web-wallet: ## build the static files of the web wallet
	$(MAKE) -C ./web-wallet all 

COMPILER_VERSION=v0.2.0
setup-compiler: ## Setup the Dusk Contract Compiler
	@./scripts/setup-compiler.sh $(COMPILER_VERSION)

.PHONY: all abi keys state wasm allcircuits contracts test bench prepare-dev run run-dev run-dev-archive help rusk rusk-wallet web-wallet setup-compiler
