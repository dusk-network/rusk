#!/bin/bash

# Function to run a single node
run_node() {
    local BOOTSTRAP_ADDR="$1"
    local PUBLIC_ADDR="$2"
    local LOG_LEVEL="$3"
    local KEYS_PATH="$4"
    local ID="$5"
    local TEMPD="$6"
    local WS_LISTEN_ADDR="$7"
    local TELEMETRY_LISTEN_ADDR="$8"
    
    local NODE_FOLDER="${TEMPD}/node_${ID}"
    local RUSK_STATE_PATH="${NODE_FOLDER}/state"

    RUSK_STATE_PATH="${RUSK_STATE_PATH}" cargo r --release -p rusk -- recovery-state --init "$GENESIS_PATH"
    
    echo "Starting node $ID ..."
    RUSK_STATE_PATH="${RUSK_STATE_PATH}" $TOOL_BIN \
    ./target/release/rusk \
            --kadcast-bootstrap "$BOOTSTRAP_ADDR" \
            --kadcast-public-address "$PUBLIC_ADDR" \
            --log-type "json" --log-level "$LOG_LEVEL" \
            --log-filter "dusk_consensus=debug" \
            --consensus-keys-path "${KEYS_PATH}/node_$ID.keys" \
            --db-path "$NODE_FOLDER" \
            --http-listen-addr "$WS_LISTEN_ADDR" \
            --telemetry-listen-addr "$TELEMETRY_LISTEN_ADDR" \
            --delay-on-resp-msg 10 \
            > "${TEMPD}/node_${ID}.log" &
}

# Cleanup function to stop all running rusk-node processes
cleanup() {
    echo "Stopping all running rusk-node processes..."
    killall rusk || true
    rm -rf "$TEMPD"
    echo "Cleanup complete."
}

# Set up trap to execute cleanup function
trap cleanup INT TERM EXIT

# Kill all existing rusk processes
killall rusk

# Determine number of pre-loaded provisioners and mode
PROV_NUM="$1"

# Set paths and addresses
GENESIS_PATH="./rusk-recovery/config/testnet.toml"
BOOTSTRAP_ADDR="127.0.0.1:7000"
DUSK_WALLET_DIR="${DUSK_WALLET_DIR:-}" # Default value if DUSK_WALLET_DIR is not set

# Check if DUSK_WALLET_DIR is set
if [ -z "$DUSK_WALLET_DIR" ]; then
    echo "Warning: DUSK_WALLET_DIR is not set"
fi

# Create a temporary directory
TEMPD=$(mktemp -d -t rust-harness.XXXX) || { echo "Failed to create temp directory"; exit 1; }
echo "Test harness folder: $TEMPD"

# Spawn nodes
for ((i = 0; i < PROV_NUM; i++)); do
    PORT=$((7000 + $i))
    WS_PORT=$((8000 + $i))
    TELEMETRY_PORT=$((9000 + $i))
    TOOL_BIN=""
    if [ $i -eq 0 ]; then
      # Run heap profiling on node-0 if heaptrack is installed
      if which heaptrack >/dev/null 2>&1; then
         TOOL_BIN="heaptrack -o /tmp/heaptrack/node-0-heap-profile"
      fi
    fi

    run_node "$BOOTSTRAP_ADDR" "127.0.0.1:$PORT" "info" "$DUSK_WALLET_DIR" "$i" "$TEMPD" "127.0.0.1:$WS_PORT" "127.0.0.1:$TELEMETRY_PORT" "$TOOL_BIN" &
done

# Monitor nodes
sleep 10
tail -F "${TEMPD}/node_*.log" | grep -e "accepted\|ERROR\|gen_candidate"

