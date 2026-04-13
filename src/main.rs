use clap::Parser;
use confluence_agent::cli::{Cli, OutputFormat};
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the tracing subscriber.
///
/// - All output goes to stderr (D-07)
/// - Default level: warn (D-08)
/// - With --verbose: debug level (D-08)
/// - Human-readable format with timestamps and span names (D-06)
fn init_tracing(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    fmt()
        .with_env_filter(EnvFilter::new(level))
        .with_writer(std::io::stderr)
        .init();
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let output_format = cli.output.clone();
    let verbose = cli.verbose;

    init_tracing(verbose);

    let result = confluence_agent::run(cli).await;

    match output_format {
        OutputFormat::Json => {
            // D-03: In JSON mode, all output (including errors) goes to stdout
            // D-05: Silent during execution; emit JSON object on completion
            let json_value = match result {
                Ok(ref cmd_result) => confluence_agent::result_to_json(cmd_result),
                Err(ref e) => confluence_agent::error_to_json(e),
            };
            println!("{}", json_value);

            // D-09: Exit code 1 on failure, 0 on success
            if result.is_err() {
                std::process::exit(1);
            }
        }
        OutputFormat::Human => {
            // D-04: Silent until done; on success: one line; on failure: error to stderr
            match result {
                Ok(cmd_result) => {
                    match cmd_result {
                        confluence_agent::CommandResult::Update {
                            page_url,
                            comments_kept,
                            comments_dropped,
                        } => {
                            println!("Updated page: {page_url}");
                            if verbose {
                                eprintln!(
                                    "  Comments kept: {comments_kept}, dropped: {comments_dropped}"
                                );
                            }
                        }
                        confluence_agent::CommandResult::Upload { page_url } => {
                            println!("Uploaded to: {page_url}");
                        }
                        confluence_agent::CommandResult::Convert { output_dir, files } => {
                            println!("Converted to: {output_dir}");
                            if verbose {
                                for f in &files {
                                    eprintln!("  {f}");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    // D-04: On failure, error message to stderr
                    eprintln!("Error: {e}");
                    // D-09: Exit code 1 on failure
                    std::process::exit(1);
                }
            }
        }
    }
}
