	#!/bin/bash
	killall rusk

	# Determines how many pre-loaded provisioners will be in use.
	PROV_NUM=$1
	MODE=$2
	#GENESIS_PATH="/home/tech/repo/dusk-network/dusk-blockchain/harness/tests/rusk_localnet_state.toml"
	GENESIS_PATH="./rusk-recovery/config/testnet.toml"

	#BOOTSTRAP_ADDR="127.0.0.1:10000"
	BOOTSTRAP_ADDR="127.0.0.1:7000"
	# DUSK_WALLET_DIR is the path to the directory containing a set of consensus keys files.
	if [ -z "$DUSK_WALLET_DIR" ]; then
	    echo "Warning: DUSK_WALLET_DIR is not set"
	fi

	# Create a temporary directory.`
	TEMPD=/tmp/rust-harness/
	rm -fr $TEMPD
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
	  local WS_LISTEN_ADDR="$7"

    local NODE_FOLDER=${TEMPD}/node_${ID}

	  local RUSK_STATE_PATH=${NODE_FOLDER}/state

	  RUSK_STATE_PATH=${RUSK_STATE_PATH} cargo r --release -p rusk -- recovery-state --init $GENESIS_PATH
	  echo "starting node $ID ..."
    echo "${KEYS_PATH}/node_$ID.keys"
	  RUSK_STATE_PATH=${RUSK_STATE_PATH} ./target/release/rusk --kadcast-bootstrap "$BOOTSTRAP_ADDR" --kadcast-public-address "$PUBLIC_ADDR" --log-level="$LOG_LEVEL" --log-filter="dusk_consensus=debug"  --consensus-keys-path="${KEYS_PATH}/node_$ID.keys" --db-path="$NODE_FOLDER" --http-listen-addr "$WS_LISTEN_ADDR" --delay-on-resp-msg=10 > "${TEMPD}/node_${ID}.log" &
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
	  --kadcast_bootstrap "$BOOTSTRAP_ADDR" --kadcast_public_address "$PUBLIC_ADDR" --log-level="$LOG_LEVEL" --log-filter="dusk_consensus=debug" \
  --consensus-keys-path="${KEYS_PATH}/node_$ID.keys" --db-path="${TEMPD}/db/${ID}" --config="default.config.toml" > "${TEMPD}/node_${ID}.log" &
}

# Spawn N (up to 9) nodes
for (( i=0; i<$PROV_NUM; i++ ))
do
  PORT=$((7000+$i))
  WS_PORT=$((8000+$i))

  #delay=$(($i))
  #sleep 20

  run_node "$BOOTSTRAP_ADDR" "127.0.0.1:$PORT" "info" "$DUSK_WALLET_DIR" "$i" "$TEMPD" "127.0.0.1:$WS_PORT" &

  # wait for bootstrappers 
  # TODO
done


# monitor
sleep 10
tail -F ${TEMPD}node_*.log | grep -e "accepted\|ERROR\|gen_candidate"
# tail -F ${TEMPD}node_*.log

# Stop all running rusk-node processes when script is interrupted or terminated
trap 'killall rusk || true; rm -rf "$TEMPD"' INT TERM EXIT

# Wait for all child processes to complete
wait
