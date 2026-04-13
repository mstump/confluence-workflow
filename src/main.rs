use clap::Parser;
use confluence_agent::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match confluence_agent::run(cli).await {
        Ok(result) => {
            // Temporary: print minimal output until Plan 02 adds formatting
            match result {
                confluence_agent::CommandResult::Update { page_url, .. } => {
                    println!("Updated page: {page_url}");
                }
                confluence_agent::CommandResult::Upload { page_url } => {
                    println!("Uploaded to: {page_url}");
                }
                confluence_agent::CommandResult::Convert { output_dir, .. } => {
                    println!("Converted to: {output_dir}");
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
