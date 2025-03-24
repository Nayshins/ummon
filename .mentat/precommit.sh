#!/bin/bash

# Enforce strict error checking
set -e

# Check if Rust is available
if ! command -v cargo &> /dev/null; then
    echo "ERROR: Rust is not available in this environment."
    echo "Cannot run precommit checks that require Rust."
    echo "Please run the setup.sh script first to install Rust."
    exit 1
fi

echo "=== Running precommit checks for Ummon ==="

# Format code - this will apply formatting changes
echo "Running cargo fmt..."
cargo fmt

# Run clippy with autofix for common issues
echo "Running cargo clippy with fixes..."
cargo clippy --fix --allow-dirty --allow-staged -- -D warnings

# Verify that the code compiles
echo "Running cargo check..."
cargo check --all-features --all-targets

# We don't run tests here since they're already in GitHub CI
# If you need tests to run, uncomment the following:
# echo "Running critical tests only..."
# cargo test --lib -- --skip=slow_

echo "=== Precommit checks completed successfully ==="
