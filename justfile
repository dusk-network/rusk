# See https://just.systems/man/en

##
## Configuration
##

export RUST_BACKTRACE := env_var_or_default("RUST_BACKTRACE", "short")
export DUSK_CONSENSUS_KEYS_PASS := env_var_or_default("DUSK_CONSENSUS_KEYS_PASS", "password")
#tmpdir  := `mktemp -d`
db_name := "temp.sqlite3"
export DATABASE_URL := env_var_or_default("DATABASE_URL", "sqlite:///tmp/" + db_name)

# By default = development mode
profile := if env_var_or_default("RELEASE", "") == "" { "debug" } else { "release" }

##
## Recipes:
##

# Drop DB & Fix SQL
reset-sql:
    @cd node/ \
    && sqlx database create \
    && sqlx database reset \
    && sqlx migrate run \
    && cargo sqlx prepare -- --all-targets --all-features || echo "Install with 'cargo install sqlx-cli --features openssl-vendored'" \
    && echo "{{DATABASE_URL}} got reset & query data updated"

# ## Build circuit keys (equivalent to make keys in root directory)
[doc('Build keys')]
keys:
    @cd rusk && cargo r \
        --no-default-features \
        --features recovery-keys \
        --release \
        -- recovery keys


# Build wasm (equivalent to make wasm in root directory)
[doc('Build wasm')]
wasm:
    make setup-compiler
    make -C ./contracts $@
    make -C ./wallet-core $@

# Copy example consensus.keys
examples:
    mkdir -p ~/.dusk/rusk
    cp examples/consensus.keys ~/.dusk/rusk/consensus.keys

# Create genesis state. Don't override existing state
genesis:
    #!/usr/bin/env bash
    # check if file exists
    if [ ! -f /tmp/example.state ]; then
        cargo r --release -p rusk -- recovery state --init examples/genesis.toml -o /tmp/example.state
    else
        echo "Genesis state already exists"
        echo "run 'just renew-genesis' if you want to replace it"
    fi

# Create genesis state. Delete any existing state
renew-genesis:
    rm /tmp/example.state # delete old state
    cargo r --release -p rusk -- recovery state --init examples/genesis.toml -o /tmp/example.state


# All preparation steps to run rusk
prepare: keys wasm examples genesis

# Run a local ephemeral archive node
archive: prepare
    @echo "Running local ephemeral archive node"
    cargo r --release --features archive -p rusk  -- -s /tmp/example.state

# Launch a local ephemeral node
node: prepare
    @echo "Running local ephemeral node"
    cargo r --release -p rusk -- -s /tmp/example.state
