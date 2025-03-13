#!/bin/bash

# Check if rustup is available and update if it is
if command -v rustup &> /dev/null; then
    echo "Updating Rust toolchain..."
    rustup update stable
else
    echo "Rustup not found, skipping toolchain update."
    echo "Using system-provided Rust installation."
fi

# Build the project to download and compile dependencies
echo "Building project..."
cargo build

echo "Setup complete!"
