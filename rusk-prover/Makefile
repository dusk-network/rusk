all: ## build 
	cargo build --release

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: $(SUBDIRS)
	cargo test --release
	cargo test --release --no-default-features
			
clippy: ## Run clippy
	@cargo clippy --release -- -D warnings
	@cargo clippy --release --no-default-features -- -D warnings

.PHONY: all help test clippy
