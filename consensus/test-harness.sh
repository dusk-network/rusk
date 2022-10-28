#!/bin/bash
killall node

# Determines how many pre-loaded provisioners will be in use.
PROV_NUM=$1
BOOTSTRAP_ADDR="127.0.0.1:7000"

# Create a temporary directory.
TEMPD=$(mktemp -d)


# Exit if the temp directory wasn't created successfully.
if [ ! -e "$TEMPD" ]; then
    >&2 echo "Failed to create temp directory"
    exit 1
fi

echo "test harness folder:$TEMPD"

# Bootstrap node with provisioner id 0
echo "Run bootstrap node"
cargo run  --example node --release -- --bootstrap $BOOTSTRAP_ADDR --provisioner-unique-id=0 --preloaded-num=$PROV_NUM --address $BOOTSTRAP_ADDR --log-level=info > "$TEMPD/node_0.log" &
sleep 3

# Spawn N (up to 9) nodes
for (( i=1; i<$PROV_NUM; i++ ))
do
  echo "Run devnet node_$i ..."
  PORT=$((7000+$i))
  cargo run  --example node --release -- --bootstrap $BOOTSTRAP_ADDR --provisioner-unique-id=$i --preloaded-num=$PROV_NUM --address "127.0.0.1:$PORT" --log-level=info > "$TEMPD/node_$i.log" &
done

# monitor
tail -F $TEMPD/node_*.log  | grep -i block_time

