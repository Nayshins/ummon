#!/bin/bash

# Exit on error
set -e

echo "=== Running precommit checks for Ummon ==="

# Ensure Rust is available
if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        echo "Sourcing Rust environment from $HOME/.cargo/env"
        source "$HOME/.cargo/env"
    elif [ -f "/root/.cargo/env" ]; then
        echo "Sourcing Rust environment from /root/.cargo/env"
        source "/root/.cargo/env"
    fi
    
    # If still not available, fail
    if ! command -v cargo &> /dev/null; then
        echo "ERROR: Rust is not available in this environment."
        echo "Please run .mentat/setup.sh first or ensure Rust is installed."
        exit 1
    fi
fi

# Print diagnostic information
echo "System information:"
echo "  - Rust: $(cargo --version)"
echo "  - rustfmt: $(command -v rustfmt || echo 'Not found')"
echo "  - clippy: $(command -v cargo-clippy || echo 'Not found')"

# Format the code
echo "Running cargo fmt..."
cargo fmt || {
    echo "ERROR: cargo fmt failed."
    echo "Please format your code with 'cargo fmt' before committing."
    exit 1
}

# Check if the formatting is correct (matches GitHub Actions workflow)
echo "Checking code formatting..."
cargo fmt -- --check || {
    echo "ERROR: Code formatting check failed."
    echo "Please format your code with 'cargo fmt' before committing."
    exit 1
}

# Run clippy with fixes
echo "Running cargo clippy with fixes..."
cargo clippy --fix --allow-dirty --allow-staged || {
    echo "ERROR: Clippy found issues that couldn't be automatically fixed."
    echo "Please fix the remaining issues before committing."
    exit 1
}

# Run clippy again to ensure all issues are fixed
echo "Verifying all clippy issues are fixed..."
cargo clippy || {
    echo "ERROR: Clippy still found issues."
    echo "Please fix all clippy warnings before committing."
    exit 1
}

# Perform a basic build check
echo "Running cargo check..."
cargo check || {
    echo "ERROR: cargo check failed."
    echo "Please fix build issues before committing."
    exit 1
}

echo "=== All precommit checks passed successfully ==="
# Exit with success only if all checks passed
exit 0
