.PHONY: all help host hosted 

all: host

check: hosted ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check
	
hosted: ## Build the WASM files
	@mkdir -p ./target/verifier-keys/
	@cp -fr ~/.rusk/keys/bid-circuits/**/*.vk ./target/verifier-keys
	@cargo rustc \
		--release \
		--target wasm32-unknown-unknown \
		--no-default-features \
		-- -C link-args=-s

test: hosted ## Perform the contract tests defined in the host module
	@cargo test \
		--release \
		--features host

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
