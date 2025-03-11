SUBDIRS := data-driver

all: $(SUBDIRS) ## Build all data drivers

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: $(SUBDIRS) ## Run all the tests in the subfolder

wasm: $(SUBDIRS) ## Build the data-driver WASM files

clippy: $(SUBDIRS) ## Run clippy

$(SUBDIRS):
	$(MAKE) -C $@ $(MAKECMDGOALS)

.PHONY: all test wasm clippy help $(SUBDIRS)
