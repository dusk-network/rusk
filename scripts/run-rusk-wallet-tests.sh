#!/bin/sh

# This script runs rusk wallet tests against a live node.
# It is run from the rusk-wallet directory, so all paths
# are relative from there.

set -e

: "${RUSK_MINIMUM_BLOCK_TIME:=1}"
export RUSK_MINIMUM_BLOCK_TIME

cp ../examples/consensus.keys ~/.dusk/rusk/consensus.keys

RAND_POSTFIX=$(mktemp XXXXXX -u)
STATE="/tmp/rusk-wallet-test-$RAND_POSTFIX.state"
NODE_LOG="/tmp/rusk-wallet-test-node-$RAND_POSTFIX.log"

# Build Rusk once
cargo build --release -p dusk-rusk --features archive

# Use the built binary to init state (no cargo run rebuild)
../target/release/rusk recovery state --init ../examples/genesis.toml -o "$STATE"

# Start nodes
DUSK_CONSENSUS_KEYS_PASS=password ../target/release/rusk \
    -s $STATE --http-listen-addr 127.0.0.1:0 > $NODE_LOG 2>&1 &
NODE_PID=$!
echo "The node ID: $NODE_PID"
# Wait for the node to start
sleep 2
export NODE_PORT=$(head -n 50 "$NODE_LOG" | awk -F'[: ]' '/Starting HTTP Listener/ {print $NF}')
if [ -z "$NODE_PORT" ]; then
    echo "Failed to get the node port"
    kill $NODE_PID || true
    exit -1
fi
echo "The node port: $NODE_PORT"

# Run tests
EXIT_STATUS=0
cargo test -p rusk-wallet --release --features e2e-test || EXIT_STATUS=$?

# Stop node
kill "$NODE_PID" || true

exit $EXIT_STATUS
