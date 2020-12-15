PROJECT := "transfer_contract"

RS_FILES := $(shell find . -name '*.rs')
WASM_RELEASE := "./target/wasm32-unknown-unknown/release/$(PROJECT).wasm"



.PHONY: all help host hosted 

all: host

check: hosted ## Run the Rust check on the project features
	@cargo check --features host
	@cargo check --features hosted
	
hosted: ## Build the WASM files
	@cargo rustc \
		--manifest-path=./Cargo.toml \
		--features hosted \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

host: hosted ## Perform the build for the unconstrained host environment
	@cargo build \
		--release \
		--features host


test: hosted ## Perform the contract tests defined in the host module
	$(MAKE) hosted
	@cargo test \
		--release \
		--features host
		

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
