mod cli;
mod commands;
mod graph;
mod parser;
mod prompt;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// We need an async main function for the async code
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Index { 
            path, 
            enable_domain_extraction, 
            domain_dir 
        } => commands::index::run(&path, enable_domain_extraction, &domain_dir).await?,
        cli::Commands::Query { query, format } => commands::query::run(&query, &format).await?,
        cli::Commands::Assist { instruction } => commands::assist::run(&instruction)?,
    }

    Ok(())
}
