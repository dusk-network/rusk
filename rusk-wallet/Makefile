DEV_WALLET_DIR ?= $(HOME)/.dusk/rusk-wallet-local-staked
DEV_WALLET_SECOND_DIR ?= $(HOME)/.dusk/rusk-wallet-local-unstaked
DEV_WALLET_PASSWORD ?= password
DEV_WALLET_CMD ?=

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the wallet
	cargo b --release

install: build ## Install the wallet
	cargo install --path .

run-dev: ## Run the staked localnet example wallet
	@if [ ! -f "$(DEV_WALLET_DIR)/wallet.dat" ]; then \
		mkdir -p "$(DEV_WALLET_DIR)"; \
		cp ../examples/wallet.dat "$(DEV_WALLET_DIR)/wallet.dat"; \
	fi
	@RUSK_WALLET_PWD="$(DEV_WALLET_PASSWORD)" cargo r --release -- \
		--network local \
		--wallet-dir "$(DEV_WALLET_DIR)" \
		$(DEV_WALLET_CMD)

run-dev-second: ## Run the funded non-staked localnet example wallet
	@if [ ! -f "$(DEV_WALLET_SECOND_DIR)/wallet.dat" ]; then \
		mkdir -p "$(DEV_WALLET_SECOND_DIR)"; \
		cp ../examples/wallet-unstaked.dat "$(DEV_WALLET_SECOND_DIR)/wallet.dat"; \
	fi
	@RUSK_WALLET_PWD="$(DEV_WALLET_PASSWORD)" cargo r --release -- \
		--network local \
		--wallet-dir "$(DEV_WALLET_SECOND_DIR)" \
		$(DEV_WALLET_CMD)

clippy: 
	cargo clippy --all-features --release -- -D warnings

test: ## Run wallet tests
	@../scripts/run-rusk-wallet-tests.sh

.PHONY: build clippy test help install run-dev run-dev-second
