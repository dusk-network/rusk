SUBDIRS := $(wildcard ./*/.)

all: $(SUBDIRS) ## Build all in the subfolder

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: ## Run all the tests for the circuits
	$(MAKE) -C ./license $@

clippy: $(SUBDIRS) ## Run clippy

doc: $(SUBDIRS) ## Run doc gen

$(SUBDIRS):
	$(MAKE) -C $@ $(MAKECMDGOALS)

.PHONY: all test help $(SUBDIRS)
