//! search command - full-text search using FTS5.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (FTS5 syntax)
    pub query: String,

    /// Maximum number of results
    #[arg(short, long, default_value = "20")]
    pub limit: usize,

    /// Rebuild the search index before searching
    #[arg(long)]
    pub rebuild: bool,
}

#[derive(Serialize)]
struct SearchOutput {
    query: String,
    count: usize,
    results: Vec<SearchResultOutput>,
}

#[derive(Serialize)]
struct SearchResultOutput {
    path: String,
    snippet: String,
    rank: f64,
}

pub fn run(args: SearchArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    // Optionally rebuild index
    if args.rebuild {
        if !output.is_json() {
            println!("Rebuilding search index...");
        }
        backend.rebuild_search_index()?;
    }

    let results = backend.search_content(&args.query, args.limit)?;

    if output.is_json() {
        let output_results: Vec<SearchResultOutput> = results
            .iter()
            .map(|r| SearchResultOutput {
                path: r.path.clone(),
                snippet: r.snippet.clone(),
                rank: r.rank,
            })
            .collect();

        output.print_json(&SearchOutput {
            query: args.query,
            count: output_results.len(),
            results: output_results,
        });
    } else {
        if results.is_empty() {
            println!("No results found for: {}", args.query);
        } else {
            println!("Found {} result(s) for: {}\n", results.len(), args.query);
            for result in results {
                println!("{}:", result.path);
                // Format snippet with highlighting markers
                let snippet = result.snippet.replace(">>>>", "\x1b[1;33m").replace("<<<<", "\x1b[0m");
                println!("  {}\n", snippet);
            }
        }
    }

    Ok(())
}
