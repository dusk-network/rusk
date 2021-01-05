REWARD_CONTRACT_DIR := "./contracts/reward"

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

wasm: ## Generate the WASM for the contract given (e.g. make wasm for=transfer)
	@cargo rustc \
		--manifest-path=contracts/$(for)/Cargo.toml \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

contracts: ## Generate the WASM for all the contracts
	  $(MAKE) -C $(REWARD_CONTRACT_DIR) all

test: ## Run the tests
		@make contracts && \
			cargo test --release -- --nocapture  && \
				rm /tmp/rusk_listener_*
		# cd contracts/bid/circuits && cargo test --release
		# cd contracts/transfer/circuits && cargo test --release

run: ## Run the server
		@make contracts && \
			cargo run --release

.PHONY: help wasm contracts test run
