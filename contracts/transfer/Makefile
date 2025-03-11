TARGET_DIR:="../../target/dusk"

all: ## Build the transfer contract
	@cargo build --release

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

check: wasm ## Run the Rust check on the project features
	@cargo check --target wasm32-unknown-unknown
	@cargo check

test: wasm ## Perform the contract tests defined in the host module
	@cargo test --release

wasm: ## Build the WASM files
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
	CARGO_TARGET_DIR=$(TARGET_DIR) \
	cargo +dusk build \
		--release \
		--color=always \
		-Z build-std=core,alloc \
		--features="contract" \
		--target wasm64-unknown-unknown

data-driver: ## Build the data-driver WASM files using wasm-bindgen and wasm-opt
	@cargo build \
		--release \
		--target wasm32-unknown-unknown \
		--bin transfer-contract-dd \
		--features="data-driver"

	@wasm-bindgen \
		--target nodejs \
		--out-dir ../../target/data-driver/transfer-contract \
		../../target/wasm32-unknown-unknown/release/transfer-contract-dd.wasm

	@echo "WASM data-driver generated in: target/data-driver/transfer-contract"
			
clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy -Z build-std=core,alloc --release --features="contract" --target wasm32-unknown-unknown -- -D warnings
	@cargo clippy --release --target wasm32-unknown-unknown --features="data-driver" --bin transfer-contract-dd -- -D warnings

doc: ## Run doc gen
	@cargo doc --release

.PHONY: all check test wasm help data-driver
