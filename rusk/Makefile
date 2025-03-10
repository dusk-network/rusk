INITFILE?=../rusk-recovery/config/localnet.toml

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: build-bench ## Run Rusk tests
	@cargo test \
		--release \
		--features testwallet \
		-- --nocapture
			
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings
	@cargo clippy \
		--no-default-features \
		--features ephemeral \
		--release \
		-- -D warnings
	@cargo clippy \
		--no-default-features \
		--features recovery-keys \
		--release \
		-- -D warnings
	@cargo clippy \
		--no-default-features \
		--features recovery-state \
		--release \
		-- -D warnings
	@cargo clippy \
		--no-default-features \
		--features prover \
		--release \
		-- -D warnings
	@cargo clippy \
		--no-default-features \
		--features chain \
		--release \
		-- -D warnings
	@cargo clippy \
		--no-default-features \
		--features archive \
		--release \
		-- -D warnings
	@cargo check --benches --features testwallet

doc: ## Run doc gen
	@cargo doc --release

build: ## Build rusk binary
	@cargo build --release

build-bench: ## Build the benchmarks
	@cargo bench --features testwallet --no-run

bench: ## Run the benchmarks
	@cargo bench --features testwallet

run:
	@cargo r --release --bin rusk

recovery-keys: ## Build prover keys
	@cargo r \
		--no-default-features \
		--features recovery-keys \
		--release \
		-- recovery keys

recovery-state: ## Build network state
	@cargo r \
		--no-default-features \
		--features recovery-state \
		--release \
		-- recovery state \
		--force \
		--init $(INITFILE)

.PHONY: test help clippy build build-bench bench recovery-keys recovery-state rusk
