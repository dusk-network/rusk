help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: ## Run Node tests
	@cargo test --release -- --nocapture

binary: ## Build a binary
	@cargo b --release --bin rusk-node

debug_with_telemetry: # Build a binary with telemetry enabled. Currently, telemetry is only based on tokio_console
	 cargo --config 'build.rustflags = ["--cfg", "tokio_unstable"]' build --features with_telemetry --bin rusk-node

clean:
	@cargo clean
			
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

.PHONY: test help binary clean
