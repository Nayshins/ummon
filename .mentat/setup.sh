#!/bin/bash

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
