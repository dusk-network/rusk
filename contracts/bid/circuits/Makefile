PROJECT := "bid_circuits"

.PHONY: all help 

all: host

test: hosted ## Run the tests for the circuits
	@cargo test \
		--release \
		

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
