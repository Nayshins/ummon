#!/bin/bash

# Install system dependencies
if command -v apt-get &> /dev/null; then
    # Debian/Ubuntu
    echo "Installing system dependencies using apt..."
    apt-get update
    apt-get install -y pkg-config libssl-dev
elif command -v yum &> /dev/null; then
    # RHEL/CentOS/Fedora
    echo "Installing system dependencies using yum..."
    yum install -y pkg-config openssl-devel
elif command -v apk &> /dev/null; then
    # Alpine
    echo "Installing system dependencies using apk..."
    apk add pkgconfig openssl-dev
elif command -v dnf &> /dev/null; then
    # Newer Fedora
    echo "Installing system dependencies using dnf..."
    dnf install -y pkg-config openssl-devel
else
    echo "Warning: Could not determine package manager to install dependencies."
    echo "You may need to manually install: pkg-config, libssl-dev"
fi

# Install Rust if not already installed
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Check Rust version
rustc --version
cargo --version

# Make sure we have necessary tools
rustup component add clippy rustfmt

# Build the project to download dependencies
cargo build
