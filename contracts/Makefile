SUBDIRS := stake-types transfer-types alice bob charlie transfer stake governance license

all: $(SUBDIRS) ## Build all the contracts

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: $(SUBDIRS) ## Run all the tests in the subfolder

wasm: $(SUBDIRS) ## Generate the WASM for all the contracts

clippy: $(SUBDIRS) ## Run clippy

$(SUBDIRS):
	$(MAKE) -C $@ $(MAKECMDGOALS)

.PHONY: all test help $(SUBDIRS)
