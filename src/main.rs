mod cli;
mod commands;
mod graph;
mod mcp_core;
mod mcp_server;
mod parser;
mod prompt;

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
            type_filter,
            path,
            exact,
            limit,
            no_llm,
            llm_provider,
            llm_model,
        } => {
            commands::query::run(
                &query,
                &format,
                type_filter.as_deref(),
                path.as_deref(),
                exact,
                limit,
                no_llm,
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
        cli::Commands::Serve { transport } => commands::serve::run(&transport).await?,
    }

    Ok(())
}
