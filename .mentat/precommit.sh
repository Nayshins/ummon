#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status

# Format code
echo "Running cargo fmt..."
cargo fmt

# Run clippy with fixes
echo "Running cargo clippy with fixes..."
cargo clippy --fix --allow-dirty --allow-staged

# Verify that the code compiles
echo "Running cargo check..."
cargo check
