help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

wasm: ## Build the WASM files
	@mkdir -p ./target/verifier-keys/
	@rm -fr ./target/verifier-keys/*.vk
	@cp -fr ~/.rusk/keys/transfer-circuits/**/*.vk ./target/verifier-keys/
	@cargo rustc \
		--release \
		--target wasm32-unknown-unknown \
		--no-default-features \
		-Zfeatures=itarget \
		-- -C link-args=-s

check: wasm ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown \
		--no-default-features \
		-Zfeatures=itarget
	@cargo check

test: wasm ## Perform the contract tests defined in the host module
	@cargo test -vvv --release
		

.PHONY: wasm check test help
