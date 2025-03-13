#!/bin/bash

# Ensure Rust toolchain is up to date
rustup update stable

# Build the project to download and compile dependencies
cargo build

echo "Setup complete!"
