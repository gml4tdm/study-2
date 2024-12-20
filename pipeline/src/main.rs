use std::path::PathBuf;
use clap::Parser;
use crate::languages::Language;
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
mod processing;

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
    AddSourceInformationToTriples(AddSourceInformationToTriplesCommand),
    GraphsToDot(GraphsToDotCommand),
    AsPredictorFeaturesToJson(AsPredictorFeaturesToJsonCommand),
    ProcessHistory(ProcessHistoryCommand),
    GenerateTimeSeriesFeatures(GenerateTimeSeriesFeaturesCommand),
    GenerateCoChangeFeatures(GenerateCoChangeFeaturesCommand),
    SummariseTriplePerformance(SummariseTriplePerformanceCommand),
    FinaliseCoChangeFeatures(FinaliseCoChangeFeaturesCommand),
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
    
    #[clap(long)]
    short: bool
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
    mapping: RenameMapping,
    
    #[clap(short, long)]
    language: Language
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

#[derive(clap::Args)]
struct AddSourceInformationToTriplesCommand {
    #[clap(short, long, num_args = 1..)]
    inputs: Vec<PathBuf>,
    
    #[clap(short, long)]
    output: Option<PathBuf>,
    
    #[clap(short, long)]
    source_directory: PathBuf,
}

#[derive(clap::Args)]
struct GraphsToDotCommand {
    #[clap(short, long, num_args = 1..)]
    input_files: Vec<PathBuf>,
    
    #[clap(short, long)]
    output_directory: PathBuf,
    
    #[clap(short, long)]
    package_diagrams: bool,
}

#[derive(clap::Args)]
struct AsPredictorFeaturesToJsonCommand {
    #[clap(short, long)]
    graph_file: PathBuf,
    
    #[clap(short, long)]
    similarity_file: PathBuf,
    
    #[clap(short, long)]
    output_file: PathBuf,
}

#[derive(clap::Args)]
struct ProcessHistoryCommand {
    #[clap(short, long)]
    input_file: PathBuf,
    
    #[clap(short, long)]
    output_file: PathBuf,
}

#[derive(clap::Args)]
struct GenerateTimeSeriesFeaturesCommand {
    #[clap(short, long, num_args = 1..)]
    input_files: Vec<PathBuf>,
    
    #[clap(short, long)]
    output_file: PathBuf,
}

#[derive(clap::Args)]
struct GenerateCoChangeFeaturesCommand {
    #[clap(short, long)]
    input_file: PathBuf,

    #[clap(short, long)]
    output_file: PathBuf
}

#[derive(clap::Args)]
struct SummariseTriplePerformanceCommand {
    #[clap(short, long, num_args = 1..)]
    input_files: Vec<PathBuf>,

    #[clap(short, long)]
    output_directory: PathBuf
}

#[derive(clap::Args)]
struct FinaliseCoChangeFeaturesCommand {
    #[clap(short, long)]
    change_file: PathBuf,
    
    #[clap(short, long, num_args = 1..)]
    graph_files: Vec<PathBuf>,
    
    #[clap(short, long)]
    output_file: PathBuf
}

fn setup_logging() -> anyhow::Result<()> {
    let spec = flexi_logger::LogSpecification::parse("warn,pipeline=trace")?;
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
            if compare.short {
                commands::compare_triple_predictions::compare_triple_predictions_short(compare.files)?;
            } else {
                commands::compare_triple_predictions::compare_triple_predictions(compare.files)?;
            }
        }
        Command::GenerateTrainTestTriples(generate) => {
            commands::generate_train_test_triples::generate_train_test_triples(
                generate.input_files, 
                generate.output_directory, 
                generate.only_common_nodes_for_training,
                generate.mapping.into_inner(),
                generate.language
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
        Command::AddSourceInformationToTriples(add) => {
            commands::add_source_information_to_triples::add_source_information_to_triples(
                add.inputs, add.source_directory, add.output
            )?;
        }
        Command::GraphsToDot(graphs_to_dot) => {
            commands::graphs_to_dot::graphs_to_dot(
                graphs_to_dot.input_files, 
                graphs_to_dot.output_directory, 
                graphs_to_dot.package_diagrams
            )?;
        }
        Command::AsPredictorFeaturesToJson(as_predictor_output_to_json) => {
            commands::as_predictor_features_to_json::as_predictor_features_to_json(
                as_predictor_output_to_json.graph_file,
                as_predictor_output_to_json.similarity_file,
                as_predictor_output_to_json.output_file
            )?;
        }
        Command::ProcessHistory(process_history) => {
            commands::process_history::process_history(
                process_history.input_file,
                process_history.output_file
            )?;
        }
        Command::GenerateTimeSeriesFeatures(generate_time_series_features) => {
            commands::generate_time_series_features::generate_time_series_features(
                generate_time_series_features.input_files,
                generate_time_series_features.output_file
            )?;
        },
        Command::GenerateCoChangeFeatures(cmd) => {
            commands::generate_co_change_features::generate_co_change_features(
                cmd.input_file, cmd.output_file
            )?;
        }
        Command::SummariseTriplePerformance(cmd) => {
            commands::summarise_triple_performance::summarise_triple_performance(
                cmd.input_files, cmd.output_directory
            )?;
        }
        Command::FinaliseCoChangeFeatures(cmd) => {
            commands::finalise_co_change_features::finalise_co_change_features(
                cmd.change_file, cmd.graph_files, cmd.output_file
            )?;
        }
    }
    
    Ok(())
}
