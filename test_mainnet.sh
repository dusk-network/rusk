#!/bin/bash
set -e
STATE_LIST_URL="https://nodes.dusk.network/state/list"

# Function to display all published states
list_states() {
  echo "Fetching available states..."
  if ! curl -f -L -s "$STATE_LIST_URL"; then
    echo "Error: Failed to fetch the list of states."
    exit 1
  fi
}

# Function to check if a specific state exists
state_exists() {
  local state=$1
  while : ; do
    if curl -f -L -s "$STATE_LIST_URL" | grep -q "^$state$"; then
      return 0 # State exists
    else
      echo "State does not exist. Please enter a state from the list below:"
      list_states
      read -p "Enter a valid state number: " state
      # Update the state_number variable in the global scope
      state_number=$state
    fi
  done
}

# Function to get the latest state
get_latest_state() {
  curl -f -L -s "$STATE_LIST_URL" | tail -n 1
}

# Check if an argument is provided, otherwise use the fallback value (348211)
if [ "$1" = "--list" ]; then
  # List all possible states
  list_states
  exit 0
elif [ -n "$1" ]; then
  # User provided a specific state, check if it exists
  state_number=$1
  state_exists "$1"
else
  # No argument provided, use the latest state
  state_number=$(get_latest_state)
fi


# Download the file
STATE_URL="https://nodes.dusk.network/state/$state_number"
echo "Downloading state $state_number from $STATE_URL..."

if ! curl -f -# -o  /tmp/state.tar.gz -L "$STATE_URL"; then
  echo "Error: Download failed. Exiting."
  #exit 1
fi

# Create a temporary directory and assign it to a variable
TEMP_DIR=$(mktemp -d)
echo "Temporary directory created at $TEMP_DIR"
tar -xvf /tmp/state.tar.gz -C $TEMP_DIR

rm -f /tmp/mainnet-genesis.state || true
cargo r --release -p dusk-rusk -- recovery state --init rusk-recovery/config/mainnet.toml -o /tmp/mainnet-genesis.state

RUSK_EXT_CHAIN=$TEMP_DIR DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p dusk-rusk -- -s /tmp/mainnet-genesis.state --config rusk/mainnet.config.toml
