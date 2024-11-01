#!/bin/bash

# Check if SKIP_CLIPPY_CHECK is set
if [ "$SKIP_CLIPPY_CHECK" = "true" ]; then
    echo "SKIP_CLIPPY_CHECK is set. Skipping clippy checks."
    exit 0
fi

echo "Detecting changes and running clippy only on modified workspace members..."

# Extract workspace members from the Cargo.toml file and members from the `make clippy` in the Makefile
WORKSPACE_MEMBERS=$(awk '/members = \[/,/\]/' ./Cargo.toml | grep -o '"[^"]*"' | tr -d '"' | sed 's/\/.*//g' | sort -u)
MAKEFILE_CLIPPY_TARGETS=$(awk '/clippy:/,/^$/' ./Makefile | grep -o '\./[^ ]*' | sed 's/\.\///;s/\/$//' | sort -u)

# Filter workspace members to include only those part of the `make clippy` target
WORKSPACE_MEMBERS=$(echo "$WORKSPACE_MEMBERS" | grep -Fxf - <(echo "$MAKEFILE_CLIPPY_TARGETS"))

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
