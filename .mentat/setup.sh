#!/bin/bash

# Check for Rust installation - this is a requirement for the project
if ! command -v cargo &> /dev/null; then
    echo "ERROR: Rust (cargo) is not installed or not in PATH"
    echo "Please install Rust before proceeding:"
    echo "1. Visit https://rustup.rs for installation instructions"
    echo "2. After installation, run 'source $HOME/.cargo/env' if needed"
    echo "3. Verify with 'cargo --version'"
    exit 1
fi

echo "Found Rust: $(cargo --version)"

# Download dependencies without building (faster)
echo "Downloading dependencies..."
cargo fetch

echo "Setup complete for Ummon development environment"
echo "To build the project: cargo build"
echo "To run tests: cargo test"
