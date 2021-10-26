 help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

wasm: ## Generate the WASM for all the contracts
	$(MAKE) -C ../test-utils $(MAKECMDGOALS)

.PHONY: wasm help $(SUBDIRS)
