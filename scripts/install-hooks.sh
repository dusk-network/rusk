#!/bin/sh
echo "Installing Git pre-push hook..."

# Check if we're in the git repository
if [ ! -d .git ]; then
    echo "This script must be run from the root of the Rusk repository."
    exit 1
fi

# Copy the pre-push.sh script to .git/hooks/pre-push
cp ./scripts/pre-push.sh .git/hooks/pre-push

echo "Git hook installed successfully."
