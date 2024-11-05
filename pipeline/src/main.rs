use std::path::PathBuf;
use clap::Parser;
use crate::utils::mapping::RenameMapping;

pub mod graphs;
pub mod utils;
mod commands;
mod file_structure;
mod languages;
mod replication;
mod datasets;
mod source_downloader;
mod statistics;

#[derive(clap::Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Diff(DiffCommand),
    ConvertASPredictorOutput(ConvertASPredictorOutputCommand),
    CompareTriplePredictions(CompareTriplePredictionsCommand),
    GenerateTrainTestTriples(GenerateTrainTestTriplesCommand),
    DownloadSources(DownloadSourcesCommand),
    ComputeProjectEvolutionStatistics(ComputeProjectEvolutionStatisticsCommand),
}

#[derive(clap::Args)]
struct DiffCommand {
    #[clap(short, long)]
    old: PathBuf,
    #[clap(short, long)]
    new: PathBuf,
}

#[derive(clap::Args)]
struct ConvertASPredictorOutputCommand {
    #[clap(short, long, num_args = 1..)]
    inputs: Vec<PathBuf>,
    
    #[clap(short, long)]
    output: PathBuf,
}

#[derive(clap::Args)]
struct CompareTriplePredictionsCommand {
    #[clap(short, long, num_args = 1..)]
    files: Vec<PathBuf>,
}

#[derive(clap::Args)]
struct GenerateTrainTestTriplesCommand {
    #[clap(short, long, num_args = 3..)]
    input_files: Vec<PathBuf>,
    
    #[clap(short, long)]
    output_directory: PathBuf,
    
    #[clap(short, long)]
    only_common_nodes_for_training: bool,
    
    #[clap(short, long, default_value = "")]
    mapping: RenameMapping
}

#[derive(clap::Args)]
struct DownloadSourcesCommand {
    #[clap(short, long)]
    input_file: PathBuf,
    
    #[clap(short, long)]
    output_directory: PathBuf,
}

#[derive(clap::Args)]
struct ComputeProjectEvolutionStatisticsCommand {
    #[clap(short, long, num_args = 1..)]
    files: Vec<PathBuf>,
    
    #[clap(short, long)]
    output: PathBuf,
    
    #[clap(short, long)]
    package_graph: bool,
}


fn setup_logging() -> anyhow::Result<()> {
    let spec = flexi_logger::LogSpecification::parse("warn,pipeline=debug")?;
    flexi_logger::Logger::with(spec)
        .log_to_file(
            flexi_logger::FileSpec::default()
                .directory("logs")
                .basename("pipeline")
                .use_timestamp(false),
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
        Command::ConvertASPredictorOutput(convert) => {
            commands::convert_as_predictor_output::convert_as_predictor_output(convert.inputs, 
                                                                               convert.output)?;
        }
        Command::CompareTriplePredictions(compare) => {
            commands::compare_triple_predictions::compare_triple_predictions(compare.files)?;
        }
        Command::GenerateTrainTestTriples(generate) => {
            commands::generate_train_test_triples::generate_train_test_triples(
                generate.input_files, 
                generate.output_directory, 
                generate.only_common_nodes_for_training,
                generate.mapping.into_inner()
            )?;
        }
        Command::DownloadSources(download) => {
            commands::download_sources::download_sources(download.input_file, download.output_directory)?;
        }
        Command::ComputeProjectEvolutionStatistics(compute) => {
            commands::compute_project_evolution_statistics::compute_project_evolution_statistics(
                compute.files, compute.output, compute.package_graph
            )?;
        }
    }
    
    Ok(())
}
