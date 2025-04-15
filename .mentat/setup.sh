#!/bin/bash

# Exit on error and print debug info
set -e

echo "=== Starting Ummon Development Environment Setup ==="

# Check for required system packages
# These should be installed in the Docker container, but verify
required_packages=("pkg-config" "libssl-dev" "build-essential")
missing_packages=()

for pkg in "${required_packages[@]}"; do
    if ! dpkg -s "$pkg" &> /dev/null; then
        missing_packages+=("$pkg")
    fi
done

if [ ${#missing_packages[@]} -gt 0 ]; then
    echo "Missing required packages: ${missing_packages[*]}"
    echo "Installing missing dependencies..."
    apt-get update
    apt-get install -y "${missing_packages[@]}"
fi

# Verify Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "ERROR: Rust is not installed. It should be available in the Docker container."
    echo "Please contact the repository owner or check the container setup."
    exit 1
fi

# Print diagnostic information
echo "System information:"
echo "  - OS: $(uname -s)"
echo "  - pkg-config: $(command -v pkg-config || echo 'Not found')"
echo "  - Rust: $(cargo --version || echo 'Not found')"
echo "  - rustfmt: $(command -v rustfmt || echo 'Not found')"
echo "  - clippy: $(command -v cargo-clippy || echo 'Not found')"

# Download dependencies
echo "Downloading project dependencies..."
cargo fetch || {
    echo "ERROR: Failed to fetch dependencies"
    exit 1
}

# Check if tree-sitter dependencies are properly installed
# These should be available in the crates.io dependencies
echo "Verifying tree-sitter package availability..."
cargo check --package tree-sitter || {
    echo "WARNING: tree-sitter package verification failed"
    # We continue anyway as this might be a temporary issue
}

# Create necessary directories
echo "Setting up environment..."
mkdir -p .vscode

echo "=== Ummon Development Environment Setup Complete ==="
echo "You can now work with the project using these commands:"
echo "- cargo build     # Build the project"
echo "- cargo test      # Run tests"
echo "- cargo fmt       # Format code"
echo "- cargo clippy    # Lint code"
echo ""
echo "Note: If you need to use OpenRouter API for LLM services,"
echo "please set the OPENROUTER_API_KEY environment variable."
