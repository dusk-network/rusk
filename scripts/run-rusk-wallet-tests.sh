#!/bin/sh

set -e

# Build rusk
cargo build --release -p dusk-rusk --features archive
cp ../target/release/rusk ../target/release/test-archiver

cargo build --release -p dusk-rusk --no-default-features --features prover
cp ../target/release/rusk ../target/release/test-prover

# Start nodes
DUSK_CONSENSUS_KEYS_PASS=password ../target/release/test-archiver  \
    -s /tmp/example.state --http-listen-addr 127.0.0.1:8080 > /tmp/test_node_logs.txt 2>&1 &
NODE_PID=$!
../target/release/test-prover --http-listen-addr 127.0.0.1:8081 > /tmp/test_prover_logs.txt 2>&1 &
PROVER_PID=$!

# Run tests
EXIT_STATUS=0
cargo test -p rusk-wallet --release --features e2e-test || EXIT_STATUS=$?

# Stop nodes
kill "$NODE_PID" "$PROVER_PID" || true

exit $EXIT_STATUS
