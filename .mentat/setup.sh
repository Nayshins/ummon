#!/bin/bash

echo "=== Starting Ummon Development Environment Setup ==="

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

# Build the project to ensure all dependencies are installed
echo "Building project to download dependencies..."
cargo build

# Install additional tools that might be useful for development
if ! command -v tree-sitter &> /dev/null; then
    echo "Installing tree-sitter CLI for grammar development..."
    cargo install tree-sitter-cli
fi

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
