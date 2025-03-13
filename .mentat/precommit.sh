#!/bin/bash

# Format code - always run this as mentioned in the development guide
cargo fmt

# Run clippy to check for code improvements with fixes where possible
cargo clippy --fix --allow-dirty --allow-staged

# Run clippy again to report any remaining issues
cargo clippy -- -D warnings

# Run a basic compilation check
cargo check
