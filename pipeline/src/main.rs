use std::path::PathBuf;
use clap::Parser;

pub mod graphs;
pub mod utils;
mod commands;

#[derive(clap::Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Diff(DiffCommand),
}

#[derive(clap::Args)]
struct DiffCommand {
    #[clap(short, long)]
    old: PathBuf,
    #[clap(short, long)]
    new: PathBuf,
}


fn setup_logging() -> anyhow::Result<()> {
    let spec = flexi_logger::LogSpecification::parse("warn,pipeline=debug")?;
    flexi_logger::Logger::with(spec)
        .log_to_file(
            flexi_logger::FileSpec::default()
                .directory("logs")
                .basename("pipeline")
                .use_timestamp(true),
        )
        .duplicate_to_stdout(flexi_logger::Duplicate::Info)
        .format_for_files(flexi_logger::detailed_format)
        .format_for_stdout(flexi_logger::colored_detailed_format)
        .set_palette("b1;3;2;4;6".to_string())
        .start()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    setup_logging()?;
    log::info!("Starting pipeline!");
    
    let cli = Cli::parse();

    match cli.command {
        Command::Diff(diff) => {
            commands::diff::diff_graph_commnd(diff.old, diff.new)?;
        }
    }
    
    Ok(())
}
