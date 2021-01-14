PROJECT := rusk

BID_CONTRACT_DIR := "./contracts/bid"
TRANSFER_CONTRACT_DIR := "./contracts/transfer"
BID_CIRCUITS_DIR := "./contracts/bid/circuits"
TRANSFER_CIRCUITS_DIR := "./contracts/transfer/circuits"
LISTENER := "/tmp/rusk_listener_pki"

all: test

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

contracts: ## Compile to WASM and test the contracts
	$(MAKE) -C $(BID_CONTRACT_DIR) test 
	##$(MAKE) -C $(TRANSFER_CONTRACT_DIR) test

circuits: ## Compile and run circuits tests
	$(MAKE) -C $(BID_CIRCUITS_DIR) test 
	$(MAKE) -C $(TRANSFER_CIRCUITS_DIR) test

test: ## Run the tests for the entire rusk repo
	@cargo build --release
	@make contracts
	@make circuits
	@cp ~/.rusk/keys/bid-circuits/0.1.0/*.pk tests/contracts/
	@cargo test 
		--release \
		-- --nocapture 
	rm $(LISTENER)

run: ## Run the server
		@make contracts && \
			cargo run --release

.PHONY: all help wasm contracts test run
