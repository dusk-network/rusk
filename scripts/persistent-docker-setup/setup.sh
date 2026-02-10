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

SELECTED_NETWORK=""
case "$NETWORK" in
    mainnet)
        SELECTED_NETWORK="mainnet"
        ;;
    testnet)
        SELECTED_NETWORK="testnet"
        ;;
    *)
        echo "Unknown network $NETWORK. Defaulting to mainnet"
        SELECTED_NETWORK="mainnet"
esac

toml_set() {
    file=$1
    property=$2
    value=$3

    # dasel selectors are dot-prefixed (e.g. ".kadcast.public_address").
    dasel put -f "$file" -r toml -t string -v "$value" ".${property}" >/dev/null
}

toml_has_table() {
    file=$1
    table=$2

    # Match `[http]` and `[http.*]` tables (keep behaviour compatible with the
    # previous `toml-cli get <file> http` check).
    grep -Eq "^\\[${table}(\\]|\\.)" "$file"
}

# Configure your local installation based on the selected network
configure_network() {
    echo "Generating configuration"

    cp "$RUSK_TEMPLATE_CONFIG_PATH" "$RUSK_CONFIG_PATH"
    toml_set "$RUSK_CONFIG_PATH" kadcast.public_address "$PUBLIC_IP:9000"
    toml_set "$RUSK_CONFIG_PATH" kadcast.listen_address "$LISTEN_IP:9000"
    if toml_has_table "$RUSK_CONFIG_PATH" http; then
        toml_set "$RUSK_CONFIG_PATH" http.listen_address "$LISTEN_IP:8080"
    fi
}

download_rusk_config_template() {
    echo "Downloading default template rusk config from the dusk node installer"
    local remote_location
    case "$SELECTED_NETWORK" in
        mainnet)
            remote_location="https://raw.githubusercontent.com/dusk-network/node-installer/ac1dd78eb31be4dba1c9c0986f6d6a06b5bd4fcc/conf/mainnet.toml"
            ;;
        testnet)
            remote_location="https://raw.githubusercontent.com/dusk-network/node-installer/ac1dd78eb31be4dba1c9c0986f6d6a06b5bd4fcc/conf/testnet.toml"
            ;;
    esac
    mkdir -p "$RUSK_CONFIG_DIR"
    curl -o "$RUSK_TEMPLATE_CONFIG_PATH" "$remote_location"
    if [ "$(cat $RUSK_TEMPLATE_CONFIG_PATH)" = "404: Not Found" ]; then
        echo "Couldn't find the default rusk template config file. This is a bug, please file an issue."
        exit 1
    fi
}

download_genesis_config() {
    echo "Downloading the genesis config from the dusk node installer"
    local remote_location
    case "$SELECTED_NETWORK" in
        mainnet)
            remote_location="https://raw.githubusercontent.com/dusk-network/node-installer/ac1dd78eb31be4dba1c9c0986f6d6a06b5bd4fcc/conf/mainnet.genesis"
            ;;
        testnet)
            remote_location="https://raw.githubusercontent.com/dusk-network/node-installer/ac1dd78eb31be4dba1c9c0986f6d6a06b5bd4fcc/conf/testnet.genesis"
            ;;
    esac
    mkdir -p "$RUSK_CONFIG_DIR"
    curl -o "$RUSK_RECOVERY_INPUT" "$remote_location"
    if [ "$(cat $RUSK_RECOVERY_INPUT)" = "404: Not Found" ]; then
        echo "Couldn't find the genesis config file. This is a bug, please file an issue."
        exit 1
    fi
}

if [ ! -f "$RUSK_TEMPLATE_CONFIG_PATH" ]; then
    download_rusk_config_template
fi
download_genesis_config
configure_network

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
