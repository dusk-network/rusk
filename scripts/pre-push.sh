#!/bin/sh

# Check if SKIP_CLIPPY_CHECK is set
if [ "$SKIP_CLIPPY_CHECK" = "true" ]; then
    echo "SKIP_CLIPPY_CHECK is set. Skipping clippy checks."
    exit 0
fi

echo "Detecting changes and running clippy only on modified workspace members..."

# List of Rust workspace members
WORKSPACE_MEMBERS="execution-core circuits contracts rusk-abi rusk-profile rusk-recovery rusk-prover node-data consensus node wallet-core rusk rusk-wallet"

# Find the common ancestor between the feature branch and master
BASE_COMMIT=$(git merge-base HEAD master)

# Get the list of changed directories since the branch point with master
CHANGED_DIRS=$(git diff --name-only "$BASE_COMMIT"...HEAD | awk -F'/' '{print $1}' | sort -u)

# Variable to track if any `clippy` checks have failed
CLIPPY_FAILED=0

# Run clippy for each workspace member if it has changes
for MEMBER in $WORKSPACE_MEMBERS; do
    if echo "$CHANGED_DIRS" | grep -q "^${MEMBER}$"; then
        echo "Running clippy on $MEMBER..."
        if ! make -C ./$MEMBER clippy; then
            CLIPPY_FAILED=1
            echo "Clippy check failed for $MEMBER."
        fi
    else
        echo "No changes detected in $MEMBER. Skipping clippy."
    fi
done

# Exit if at least 1 clippy run failed. Exit 1 aborts, exit 0 allows for pushing 
if [ "$CLIPPY_FAILED" -eq 1 ]; then
    echo "One or more clippy checks failed. Push aborted."
    exit 1
else
    echo "All clippy checks passed. Proceeding with push."
    exit 0
fi
