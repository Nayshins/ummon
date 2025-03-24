#!/bin/bash

# Exit on error
set -e 

echo "=== Starting Ummon Development Environment Setup ==="

# Install system dependencies
if command -v apt-get &> /dev/null; then
    echo "Detected apt-based system, installing dependencies..."
    apt-get update
    apt-get install -y pkg-config libssl-dev build-essential
elif command -v dnf &> /dev/null; then
    echo "Detected dnf-based system, installing dependencies..."
    dnf install -y pkg-config openssl-devel gcc
elif command -v yum &> /dev/null; then
    echo "Detected yum-based system, installing dependencies..."
    yum install -y pkg-config openssl-devel gcc
elif command -v apk &> /dev/null; then
    echo "Detected Alpine Linux, installing dependencies..."
    apk add --no-cache pkgconfig openssl-dev build-base
elif command -v brew &> /dev/null; then
    echo "Detected macOS, installing dependencies..."
    brew install pkg-config openssl@3
    # Export OpenSSL paths for macOS
    export OPENSSL_DIR=$(brew --prefix openssl@3)
    export PKG_CONFIG_PATH="$OPENSSL_DIR/lib/pkgconfig:$PKG_CONFIG_PATH"
else
    echo "WARNING: Could not detect package manager to install dependencies."
    echo "You may need to manually install: pkg-config, OpenSSL development libraries"
fi

# Check if Rust is available 
if ! command -v cargo &> /dev/null; then
    echo "Rust not detected. Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust detected: $(cargo --version)"
    # Update Rust to ensure we have the latest toolchain
    rustup update
fi

# Print diagnostic information
echo "System information:"
echo "  - OS: $(uname -s)"
echo "  - pkg-config: $(command -v pkg-config || echo 'Not found')"
echo "  - Rust: $(cargo --version || echo 'Not found')"

# Build the project without installing additional tools first
echo "Building project to download dependencies..."
cargo build || {
    echo "Initial build failed, but we'll continue with setup."
    echo "You may need to manually install additional dependencies."
}

# Skip tree-sitter installation to avoid potential issues
# It can be installed manually if needed

echo "Setting up environment..."
# Create any necessary directories
mkdir -p .vscode
# Note: we're not creating any files here, just ensuring directories exist

echo "=== Ummon Development Environment Setup Complete ==="
echo "You can now work with the project using these commands:"
echo "- cargo build     # Build the project"
echo "- cargo test      # Run tests"
echo "- cargo fmt       # Format code"
echo "- cargo clippy    # Lint code"
echo ""
echo "Note: If you need to use OpenRouter API for LLM services,"
echo "please set the OPENROUTER_API_KEY environment variable."
echo ""
echo "Note: If you encounter build issues, you may need to manually install"
echo "additional system dependencies depending on your environment."
echo "Common requirements: pkg-config, libssl-dev/openssl-devel"
