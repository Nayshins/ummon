mod cli;
mod commands;
mod db;
mod graph;
mod mcp_core;
mod mcp_server;
mod parser;
mod prompt;
mod query;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// We need an async main function for the async code
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with stderr output for compatibility with stdin/stdout protocols
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Index {
            path,
            enable_domain_extraction,
            domain_dir,
            llm_provider,
            llm_model,
        } => {
            commands::index::run(
                &path,
                enable_domain_extraction,
                &domain_dir,
                llm_provider.as_deref(),
                llm_model.as_deref(),
            )
            .await?
        }
        cli::Commands::Query {
            query,
            format,
            no_llm,
            limit,
            llm_provider,
            llm_model,
            ..
        } => {
            // Use the natural flag as the opposite of no_llm
            let natural = !no_llm;

            commands::query::run(
                &query,
                &format,
                natural,
                limit,
                llm_provider.as_deref(),
                llm_model.as_deref(),
            )
            .await?
        }
        cli::Commands::Assist {
            instruction,
            llm_provider,
            llm_model,
        } => {
            commands::assist::run(&instruction, llm_provider.as_deref(), llm_model.as_deref())
                .await?
        }
        cli::Commands::Serve => commands::serve::run().await?,
    }

    Ok(())
}
