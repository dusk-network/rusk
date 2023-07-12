SUBDIRS := $(wildcard ./tests/contracts/*/.)

all: ## Build the ABI
	cargo build

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

test: $(SUBDIRS)
	cargo test --release
			
clippy: ## Run clippy
	@cargo clippy --release -- -D warnings

$(SUBDIRS):
	$(MAKE) -C $@

.PHONY: all help test $(SUBDIRS)
