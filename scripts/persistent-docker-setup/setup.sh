#!/bin/bash
set -e 

# Script for setting up a node in the persistent state Docker container.
#
# It detects the IP addresses for the node, generates a configuration file for
# the node based on the default `rusk.toml` used in the dusk-node-installer (or
# a user-supplied configuration file which can be provided by mounting at 
# `/opt/dusk/conf/rusk.template.toml`), and runs the node.

echo "Starting node environment"

RUSK_CONFIG_DIR=/opt/dusk/conf
RUSK_TEMPLATE_CONFIG_PATH="$RUSK_CONFIG_DIR/rusk.template.toml"
RUSK_CONFIG_PATH="$RUSK_CONFIG_DIR/rusk.toml"

detect_ips_output=$(./detect_ips.sh)
PUBLIC_IP=$(echo "$detect_ips_output" | sed -n '1p')
LISTEN_IP=$(echo "$detect_ips_output" | sed -n '2p')

toml_set() {
    file=$1
    property=$2
    value=$3

    echo -e "$(toml-cli set $file $property $value)" > $file
}

configure_network() {
    local network=$1
    local kadcast_id
    local bootstrapping_nodes
    local genesis_timestamp
    local base_state
    local prover_url

    case "$network" in
        mainnet)
            kadcast_id="0x1"
            bootstrapping_nodes="['165.232.91.113:9000', '64.226.105.70:9000', '137.184.232.115:9000']"
            genesis_timestamp="'2025-01-07T12:00:00Z'"
            base_state="https://nodes.dusk.network/genesis-state"
            prover_url="https://provers.dusk.network"
            ;;
        testnet)
            kadcast_id="0x2"
            bootstrapping_nodes="['134.122.62.88:9000','165.232.64.16:9000','137.184.118.43:9000']"
            genesis_timestamp="'2024-12-23T17:00:00Z'"
            base_state="https://testnet.nodes.dusk.network/genesis-state"
            prover_url="https://testnet.provers.dusk.network"
            ;;
        devnet)
            kadcast_id="0x3"
            bootstrapping_nodes="['128.199.32.54', '159.223.29.22', '143.198.225.158']"
            genesis_timestamp="'2024-12-23T12:00:00Z'"
            base_state="https://devnet.nodes.dusk.network/genesis-state"
            prover_url="https://devnet.provers.dusk.network"
            ;;
        *)
            echo "Unknown network: $network. Defaulting to mainnet."
            configure_network "mainnet"
            return
            ;;
    esac

    echo "Generating configuration"

    cat > "$RUSK_CONFIG_DIR/genesis.toml" <<EOF
base_state = "$base_state"
EOF

    cp "$RUSK_TEMPLATE_CONFIG_PATH" "$RUSK_CONFIG_PATH"
    sed -i "s/^kadcast_id =.*/kadcast_id = $kadcast_id/" "$RUSK_CONFIG_PATH"
    sed -i "s/^bootstrapping_nodes =.*/bootstrapping_nodes = $bootstrapping_nodes/" "$RUSK_CONFIG_PATH"
    sed -i "s/^genesis_timestamp =.*/genesis_timestamp = $genesis_timestamp/" "$RUSK_CONFIG_PATH"
    toml_set "$RUSK_CONFIG_PATH" kadcast.public_address "$PUBLIC_IP:9000"
    toml_set "$RUSK_CONFIG_PATH" kadcast.listen_address "$LISTEN_IP:9000"
    if toml-cli get "$RUSK_CONFIG_PATH" http &> /dev/null; then
        toml_set "$RUSK_CONFIG_PATH" http.listen_address "$LISTEN_IP:8080"
    fi
}

download_rusk_config() {
    echo "Downloading default template rusk config from the dusk node installer"
    REMOTE_LOCATION=https://raw.githubusercontent.com/dusk-network/node-installer/9cdf0be1372ca6cb52cb279bd58781a3a27bf8ae/conf/rusk.toml
    mkdir -p "$RUSK_CONFIG_DIR"
    curl -o "$RUSK_TEMPLATE_CONFIG_PATH" "$REMOTE_LOCATION"
    if [ "$(cat $RUSK_TEMPLATE_CONFIG_PATH)" = "404: Not Found" ]; then
        echo "Couldn't find the default rusk template config file. This is a bug, please file an issue."
        exit 1
    fi
}

if [ ! -f "$RUSK_TEMPLATE_CONFIG_PATH" ]; then
    download_rusk_config
fi

configure_network "$NETWORK"

if [ -z "$DUSK_CONSENSUS_KEYS_PASS" ]; then
    echo "DUSK_CONSENSUS_KEYS_PASS is not set"
    exit 1
fi

echo "Selected network: $NETWORK"

/opt/dusk/bin/rusk recovery keys
/opt/dusk/bin/rusk recovery state 

echo "Starting rusk"
echo "Rusk config:"
cat "$RUSK_CONFIG_PATH"
/opt/dusk/bin/rusk --config "$RUSK_CONFIG_PATH"
