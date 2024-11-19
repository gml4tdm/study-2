use std::path::PathBuf;
use crate::graphs::loaders::load_graph_from_file;
use crate::replication::as_predictor::similarities::build_edge_list;


pub fn as_predictor_features_to_json(graph_path: PathBuf,
                                     similarity_path: PathBuf,
                                     output_path: PathBuf) -> anyhow::Result<()>
{
    let graph = load_graph_from_file(&graph_path)?
        .to_module_graph();
    let annotated = build_edge_list(&graph, similarity_path)?;
    if let Some(path) = output_path.parent() {
        std::fs::create_dir_all(path)?;
    }
    let file = std::fs::File::create(output_path)?;
    serde_json::to_writer_pretty(file, &annotated)?;
    Ok(())
}