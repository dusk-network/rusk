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

test: contracts circuits ## Run the tests for the entire rusk repo
	@cargo build --release
	@cp ~/.rusk/keys/bid-circuits/0.1.0/*.pk tests/contracts/
	@cargo test 
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

clear: ## Clear Rusk circuit keys
	@cargo clean
	@rm -f Cargo.lock
	@rm -fr target
	@rm -fr ~/.rusk/keys

keys: ## Create Rusk keys
	@cargo build --release

contracts: ## Generate the WASM for all the contracts
		@for file in `find contracts -maxdepth 2 -name "Cargo.toml"` ; do \
			cargo rustc \
				--manifest-path=$${file} \
				--release \
				--target wasm32-unknown-unknown \
				-- -C link-args=-s; \
		done

test: contracts test_bid test_transfer ## Run the tests
	@cargo test --release -- --nocapture && \
		rm $((LISTENER))

test_bid: contracts keys ## Run the bid contract tests
	@cd contracts/bid/circuits && \
		cargo test --release

test_transfer: contracts keys ## Run the transfer contract tests
	@cd contracts/transfer/circuits && \
		cargo test --release

run: contracts ## Run the server
	@cargo run --release

.PHONY: all help wasm clear keys contracts circuits test test_bid test_transfer run
