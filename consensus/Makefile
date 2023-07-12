help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test-harness:
	echo "Run test-harness cluster with 8 nodes, all provisioners"
	cargo b --release; ./test-harness.sh 8

testbed:
	echo "Run testbed cluster (single-process)"
	cargo run --release --example testbed

release:
	cargo clippy; cargo b --release

test: ## Run tests
	@cargo test --release -- --nocapture
	
clippy: ## Run clippy
	@cargo clippy --all-features --release -- -D warnings

clean:
	@cargo clean

.PHONY: test help clean test-harness testbed release
