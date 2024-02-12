INITFILE?=../rusk-recovery/config/localnet.toml

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run Rusk tests
	@cargo test \
		--release \
		--features testwallet \
		-- --nocapture
			
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings
	@cargo clippy --no-default-features --release -- -D warnings
	@cargo clippy --no-default-features --features ephemeral --release -- -D warnings
	@cargo clippy --no-default-features --features recovery-keys --release -- -D warnings
	@cargo clippy --no-default-features --features recovery-state --release -- -D warnings
	@cargo clippy --no-default-features --features prover --release -- -D warnings
	@cargo clippy --no-default-features --features node --release -- -D warnings

build: ## Build rusk binary
	@cargo build --release

run:
	@cargo r --release --bin rusk

recovery-keys: ## Build circuit keys
	@cargo r --no-default-features --features recovery-keys --release -- recovery-keys

recovery-state: ## Build network state
	@cargo r --no-default-features --features recovery-state --release -- recovery-state --init $(INITFILE)

.PHONY: test help clippy build recovery-keys recovery-state rusk
