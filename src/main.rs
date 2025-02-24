mod cli;
mod commands;
mod graph;
mod parser;
mod prompt;
mod server;

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
        cli::Commands::Index { path } => commands::index::run(&path).await?,
        cli::Commands::Query { query } => commands::query::run(&query).await?,
        cli::Commands::Serve { port, host } => server::run_server(&host, port).await?,
        cli::Commands::Analyze { target, depth } => {
            let result = server::analysis::analyze_impact(&target, depth).await?;
            println!("Impact analysis results for {}:", target);
            for (id, score) in &result.impacts {
                if let Some(name) = result.entity_names.get(id.as_str()) {
                    println!("  - {} (impact score: {:.2})", name, score);
                }
            }
        },
        cli::Commands::MapDomain { concept } => {
            let result = server::domain::map_domain_to_code(&concept).await?;
            println!("Code implementations for domain concept '{}':", concept);
            for entity in &result.entities {
                println!("  - {} ({}) - relevance: {:.2}", entity.name, entity.entity_type, entity.relevance);
            }
        },
        cli::Commands::MapCode { path } => {
            let results = server::domain::map_code_to_domain(&path).await?;
            println!("Domain concepts related to '{}':", path);
            for mapping in &results {
                println!("  - {}", mapping.concept);
            }
        },
    }

    Ok(())
}
