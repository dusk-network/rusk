help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: ## Build the code-hasher macro
	@cargo build --release 

test: ## Perform the test of the code-hasher macro
	@cargo test \
		--release \

.PHONY: all test help
