#!/bin/bash
killall rusk-node

# Determines how many pre-loaded provisioners will be in use.
PROV_NUM=$1
MODE=$2
BOOTSTRAP_ADDR="127.0.0.1:7000"

# DUSK_WALLET_DIR is the path to the directory containing a set of consensus keys files.
if [ -z "$DUSK_WALLET_DIR" ]; then
    echo "Warning: DUSK_WALLET_DIR is not set"
fi

# Create a temporary directory.
TEMPD=/tmp/rust-harness/
mkdir $TEMPD

# Exit if the temp directory wasn't created successfully.
if [ ! -e "$TEMPD" ]; then
    >&2 echo "Failed to create temp directory"
    exit 1
fi

echo "test harness folder:$TEMPD"

run_node() {
  if [ "$MODE" = "--with_telemetry" ]; then
    run_node_with_telemetry "$@"
  else
    run_node_debug_mode "$@"
  fi
}

run_node_debug_mode() {
  local BOOTSTRAP_ADDR="$1"
  local PUBLIC_ADDR="$2"
  local LOG_LEVEL="$3"
  local KEYS_PATH="$4"
  local ID="$5"
  local TEMPD="$6"

  echo "starting node $ID ..."
  cargo run --bin rusk-node -- --kadcast_bootstrap "$BOOTSTRAP_ADDR" --kadcast_public_address "$PUBLIC_ADDR" --log-level="$LOG_LEVEL" --consensus-keys-path="${KEYS_PATH}/node_$ID.keys" --db-path="${TEMPD}/db/${ID}" --config="default.config.toml" > "${TEMPD}/node_${ID}.log" &
}

## Use ~/.cargo/bin/tokio-console --retain-for 0s http://127.0.0.1:10000 to connect console to first node
run_node_with_telemetry() {
  local BOOTSTRAP_ADDR="$1"
  local PUBLIC_ADDR="$2"
  local LOG_LEVEL="$3"
  local KEYS_PATH="$4"
  local ID="$5"
  local TEMPD="$6"
  
  T_BIND_PORT=$((10000+$ID))

  echo "starting node $ID ..."

  RUST_LOG="info" TOKIO_CONSOLE_BIND="127.0.0.1:$T_BIND_PORT" \
  cargo --config 'build.rustflags = ["--cfg", "tokio_unstable"]' run --features with_telemetry --bin rusk-node --\
  --kadcast_bootstrap "$BOOTSTRAP_ADDR" --kadcast_public_address "$PUBLIC_ADDR" --log-level="$LOG_LEVEL" \
  --consensus-keys-path="${KEYS_PATH}/node_$ID.keys" --db-path="${TEMPD}/db/${ID}" --config="default.config.toml" > "${TEMPD}/node_${ID}.log" &
}


# Spawn N (up to 9) nodes
for (( i=0; i<$PROV_NUM; i++ ))
do
  PORT=$((7000+$i))
  run_node "$BOOTSTRAP_ADDR" "127.0.0.1:$PORT" "info" "$DUSK_WALLET_DIR" "$i" "$TEMPD"

  # Assuming first node is the bootstrap node, we need to wait for it to start
  if [ $i -eq 0 ]; then
    sleep 3
  fi
 
done


# monitor
sleep 2
tail -F ${TEMPD}node_*.log | grep -e "accepted\|ERROR"

# Stop all running rusk-node processes when script is interrupted or terminated
trap 'killall rusk-node || true; rm -rf "$TEMPD"' INT TERM EXIT

# Wait for all child processes to complete
wait
