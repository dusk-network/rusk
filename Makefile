help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

wasm: ## Generate the WASM for the contract given (e.g. make wasm for=transfer)
	@cargo rustc \
		--manifest-path=contracts/$(for)/Cargo.toml \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

clear: ## Clear Rusk circuit keys
	@rm -fr ~/.rusk/keys

keys: ## Create Rusk keys
	@cargo check --release

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
		rm /tmp/rusk_listener_*

test_bid: contracts keys ## Run the bid contract tests
	@cd contracts/bid/circuits && \
		cargo test --release

test_transfer: contracts keys ## Run the transfer contract tests
	@cd contracts/transfer/circuits && \
		cargo test --release

run: contracts ## Run the server
	@cargo run --release

.PHONY: help wasm clear keys contracts test test_bid test_transfer run
