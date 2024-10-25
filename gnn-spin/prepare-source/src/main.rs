use std::collections::HashMap;
use std::path::PathBuf;
use clap::Parser;
use crate::hierarchy::build_hierarchy;
use crate::language::Language;
use crate::prepare::find_source_pairs;
use crate::schema::DependencyGraphRoot;
use crate::select::select_sources_from_graph;

mod schema;
mod prepare;
mod select;
mod language;
mod resolver;
mod hierarchy;

/// Command line arguments
#[derive(Debug, Clone, clap::Parser)]
struct Cli {
    /// Directory containing all the graph source files
    #[arg(short, long)]
    graph_directory: PathBuf,
    
    /// Format of the inputted graph files
    #[arg(short = 'f', long, default_value_t = GraphFormat::ODEM)]
    graph_format: GraphFormat,
    
    /// Source code directory
    #[arg(short, long)]
    source_directory: PathBuf,
    
    /// Output directory
    #[arg(short, long)]
    output_directory: PathBuf,
    
    /// Project name mapping to resolve graph paths 
    #[arg(long, default_value_t = CliMap::empty())]
    project_name_mapping: CliMap
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum GraphFormat {
    ODEM
}

impl GraphFormat {
    pub fn extension(&self) -> &str {
        match self {
            GraphFormat::ODEM => "odem"
        }
    }
}

impl std::str::FromStr for GraphFormat {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "odem" => Ok(GraphFormat::ODEM),
            _ => Err(anyhow::anyhow!("Unknown graph format: {}", s))
        }
    }
}

impl std::fmt::Display for GraphFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphFormat::ODEM => write!(f, "odem")
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliMap(HashMap<String, String>);

impl CliMap {
    pub fn into_inner(self) -> HashMap<String, String> {
        self.0
    }
    
    pub fn empty() -> Self {
        CliMap(HashMap::new())
    }
}

impl std::str::FromStr for CliMap {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut map = HashMap::new();
        for pair in s.split(';') {
            if pair.is_empty() {
                continue;
            }
            let mut split = pair.split('=');
            let key = split.next().ok_or_else(|| anyhow::anyhow!("Missing key"))?;
            let value = split.next().ok_or_else(|| anyhow::anyhow!("Missing value"))?;
            map.insert(key.to_string(), value.to_string());
        }
        Ok(CliMap(map))
    }
}

impl std::fmt::Display for CliMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut pairs = Vec::new();
        for (key, value) in self.0.iter() {
            pairs.push(format!("{}={}", key, value));
        }
        write!(f, "{}", pairs.join(";"))
    }
}


fn main() -> anyhow::Result<()> {
    let writer = logforth::append::rolling_file::RollingFileWriter::builder()
        .max_log_files(1)
        .rotation(logforth::append::rolling_file::Rotation::Never)
        .build("logs")?;
    let (nonblocking, _guard) = logforth::append::rolling_file::NonBlockingBuilder::default()
        .finish(writer);
    let file = logforth::append::rolling_file::RollingFile::new(nonblocking);
    logforth::Logger::new()
        .dispatch(
            logforth::Dispatch::new()
                .filter(log::LevelFilter::Debug)
                .layout(logforth::layout::TextLayout::default())
                .append(logforth::append::Stdout)
        )
        .dispatch(
            logforth::Dispatch::new()
                .filter(log::LevelFilter::Trace)
                .layout(logforth::layout::TextLayout::default().no_color())
                .append(file)
        )
        .apply()?;
        
    let args = Cli::parse();
    log::debug!("Graph directory: {}", args.graph_directory.display());
    log::debug!("Source directory: {}", args.source_directory.display());
    log::debug!("Graph format: {:?}", args.graph_format);
    let pairs = find_source_pairs(
        args.graph_directory,
        args.source_directory, 
        args.graph_format,
        args.project_name_mapping.into_inner()
    )?;
    for pair in pairs {
        log::info!("Processing project: {} v{}", pair.project, pair.version);
        let file = std::fs::File::open(&pair.graph)?;
        let reader = std::io::BufReader::new(file);
        let graph: DependencyGraphRoot = quick_xml::de::from_reader(reader)?;
        let sources = select_sources_from_graph(graph, Language::Java, pair.code)?;
        log::info!("Found {} source files", sources.len());
        let hierarchy = build_hierarchy(sources)?;
        let target = args.output_directory
            .join(pair.project.clone())
            .join(pair.version.clone());
        std::fs::create_dir_all(&target)?;
        let file = std::fs::File::create(target.join("hierarchy.json"))?;
        serde_json::to_writer_pretty(file, &hierarchy)?;
    }
    Ok(())
}
