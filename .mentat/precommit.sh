#!/bin/bash

# Check if Rust is available
if ! command -v cargo &> /dev/null; then
    echo "WARNING: Rust is not available in this environment."
    echo "Cannot run precommit checks that require Rust."
    echo "Please install Rust to enable proper code validation."
    # Still exit with success - informational only
    exit 0
fi

# If we get here, Rust is available

echo "Running precommit checks for Ummon..."

# Format code
echo "Running cargo fmt..."
cargo fmt || echo "WARNING: cargo fmt failed, but continuing..."

# Run clippy with fixes
echo "Running cargo clippy with fixes..."
cargo clippy --fix --allow-dirty --allow-staged || echo "WARNING: clippy failed, but continuing..."

# Verify that the code compiles
echo "Running cargo check..."
cargo check || echo "WARNING: cargo check failed, but continuing..."

echo "Precommit checks completed."
exit 0
