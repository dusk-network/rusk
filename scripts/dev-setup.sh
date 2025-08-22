#!/bin/bash
set -e

echo "Starting development environment setup for Rusk node..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
else
    SUDO=""
fi

# Detect OS package manager
if command -v apt-get &> /dev/null; then
    PKG_MANAGER="apt-get"
elif command -v pacman &> /dev/null; then
    PKG_MANAGER="pacman"
elif command -v yum &> /dev/null; then
    PKG_MANAGER="yum"
elif command -v brew &> /dev/null; then
    PKG_MANAGER="brew"
# TODO: Create musl release of Duskc
# elif command -v apk &> /dev/null; then
#     PKG_MANAGER="apk"
else
    echo "Unsupported package manager. Please install the dependencies manually."
    exit 1
fi

# Function to install dependencies based on package manager
install_dependencies() {
    case "$PKG_MANAGER" in
        apt-get)
            echo "Updating package list..."
            $SUDO apt-get update
            echo "Installing packages for Ubuntu/Debian..."
            $SUDO apt-get install -y curl zip libssl-dev gcc clang make pkg-config
            ;;
        pacman)
            echo "Installing packages for Arch Linux..."
            $SUDO pacman -Sy --noconfirm curl zip unzip base-devel openssl clang
            ;;
        yum)
            echo "Installing packages for CentOS/RHEL..."
            $SUDO yum install -y curl zip unzip openssl-devel gcc clang make pkg-config
            ;;
        brew)
            echo "Installing packages for macOS..."
            brew install curl zip openssl gcc make pkg-config
            ;;
        # apk)
        #     echo "Updating package list..."
        #     $SUDO apk update
        #     echo "Installing packages for Alpine Linux..."
        #     $SUDO apk add musl musl-dev build-base curl zip unzip openssl-dev openssl-libs-static gcc g++ libc-dev binutils gcompat clang git make pkgconf
        #     ;;
    esac
}

# Function to configure the Rust environment based on the user's shell
configure_rust_env() {
    if echo "$SHELL" | grep -q "fish"; then
        echo "Configuring Rust for fish shell..."
        source "$HOME/.cargo/env.fish"
    else
        echo "Configuring Rust for sh-compatible shell..."
        . "$HOME/.cargo/env"
    fi
}

# Check and install Rust and wasm-pack based on rust-toolchain.toml
install_rust_and_wasm_pack() {
    # Path to rust-toolchain.toml in the Rusk root directory
    TOOLCHAIN_FILE="./rust-toolchain.toml"
    
    if [ -f "$TOOLCHAIN_FILE" ]; then
        # Extract the nightly version specified in rust-toolchain.toml
        RUST_VERSION=$(grep -Eo 'channel = "(.*)"' "$TOOLCHAIN_FILE" | sed 's/channel = "//;s/"//')
    else
        echo "rust-toolchain.toml not found in the Rusk root directory. Exiting."
        exit 1
    fi

    # Check if rustc is available
    if ! command -v rustc &> /dev/null; then
        echo "Rust not found. Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        
        # Source Rust environment initially
        configure_rust_env
    else
        echo "Rust is already installed."
    fi

    echo "Setting up Rust toolchain $RUST_VERSION..."
    rustup toolchain install "$RUST_VERSION"
    rustup default "$RUST_VERSION"

    # Re-source Rust environment after toolchain installation
    configure_rust_env

    if ! command -v wasm-pack &> /dev/null; then
        echo "Installing wasm-pack..."
        cargo +nightly install wasm-pack
    else
        echo "wasm-pack is already installed."
    fi
}

# Run dependency installation
install_dependencies
echo "Dependency installation complete."

install_rust_and_wasm_pack
echo "Development environment setup complete."
echo "Restart your shell environment to apply new configurations."
