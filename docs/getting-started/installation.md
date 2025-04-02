---
title: Installation
description: How to install and set up Ummon
---

# Installing Ummon

Ummon is a Rust-based tool that can be installed using Cargo, Rust's package manager. This guide will walk you through the installation process.

## Prerequisites

Before installing Ummon, ensure you have the following installed:

- **Rust and Cargo**: Ummon is built with Rust. If you don't have Rust installed, you can install it using [rustup](https://rustup.rs/).
- **SQLite**: Ummon uses SQLite for its database. Most systems come with SQLite pre-installed.

## Installation Methods

### Installing from crates.io

The simplest way to install Ummon is directly from crates.io:

```bash
cargo install ummon
```

This will download, compile, and install the latest published version of Ummon.

### Building from Source

Alternatively, you can build Ummon from source:

```bash
# Clone the repository
git clone https://github.com/yourusername/ummon.git
cd ummon

# Build the project
cargo build --release

# The binary will be available at ./target/release/ummon
```

## Environment Setup

Ummon uses environment variables for sensitive configuration:

- `OPENROUTER_API_KEY`: API key for LLM services (required for natural language queries and domain extraction)

You can set this in your shell profile or before running Ummon commands:

```bash
export OPENROUTER_API_KEY="your-api-key-here"
```

## Verifying Installation

Verify that Ummon is correctly installed by running:

```bash
ummon --version
```

You should see the version information of your installed Ummon instance.

## Next Steps

Now that you have Ummon installed, you can:

- Move on to the [Quick Start Guide](/getting-started/quick-start.md) to begin using Ummon
- Learn about the [Knowledge Graph](/features/knowledge-graph.md) system
- Explore the [Query System](/features/query-system.md) capabilities
