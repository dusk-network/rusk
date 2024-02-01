#!/bin/sh

set -e

# The script should check whether the inputs are set, or print usage
# information otherwise.
if [ -z "$1" ]; then
    echo "Usage: $0 <compiler-version>"
    exit 1
fi

RELEASES_URL=https://github.com/dusk-network/rust/releases/download

COMPILER_VERSION=$1
COMPILER_ARCH=$(rustc -vV | sed -n 's|host: ||p')

ARTIFACT_NAME=duskc-$COMPILER_ARCH.zip
ARTIFACT_URL=$RELEASES_URL/$COMPILER_VERSION/$ARTIFACT_NAME

ARTIFACT_DIR=$PWD/target/dusk/$COMPILER_VERSION
ARTIFACT_PATH=$ARTIFACT_DIR/$ARTIFACT_NAME

# If the artifact doesn't already exist in the target directory, download it,
# otherwise skip.
if [ ! -f "$ARTIFACT_PATH" ]; then
    echo "Downloading compiler version $COMPILER_VERSION"
    mkdir -p "$ARTIFACT_DIR"
    curl -L "$ARTIFACT_URL" -o "$ARTIFACT_PATH"
fi

# Unzip the artifact, if it isn't already unzipped
UNZIPPED_DIR=$ARTIFACT_DIR/unzipped

if [ ! -d "$UNZIPPED_DIR" ]; then
    echo "Extracting compiler..."
    mkdir -p "$UNZIPPED_DIR"
    unzip "$ARTIFACT_PATH" -d "$UNZIPPED_DIR" >> /dev/null
    # We don't require the source of the compiler itself
    rm "$UNZIPPED_DIR/rustc-nightly-src.tar.gz"
fi

# Extract the tarballs, if they aren't already extracted
EXTRACTED_DIR=$ARTIFACT_DIR/extracted

if [ ! -d "$EXTRACTED_DIR" ]; then
    mkdir -p "$EXTRACTED_DIR"
    tarballs=$(find "$UNZIPPED_DIR" -name '*.tar.gz')
    for tarball in $tarballs; do
        tar -xzf "$tarball" -C "$EXTRACTED_DIR" --strip-components=2 &
    done
    wait
    # We don't require the, clearly clobbered at this point, file manifest
    rm "$EXTRACTED_DIR/manifest.in"
fi

# Ensure that the extracted compiler is symlinked in the toolchain directory
TOOLCHAIN_DIR=${RUSTUP_HOME:-$HOME/.rustup}/toolchains
TOOLCHAIN_LINK=$TOOLCHAIN_DIR/dusk

rm -f "$TOOLCHAIN_LINK"
ln -s "$EXTRACTED_DIR" "$TOOLCHAIN_LINK"
