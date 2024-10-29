use std::path::Path;
use crate::graphs::DependencyGraph;
use crate::graphs::loaders::odem::OdemGraphRoot;
use crate::graphs::loaders::rsf::FromRsfFile;

mod odem;
mod rsf;


pub fn load_odem_graph(path: impl AsRef<Path>) -> anyhow::Result<DependencyGraph> {
    let odem = OdemGraphRoot::load_from_file(path)?;
    Ok(DependencyGraph::from(odem))
}

pub fn load_rsf_graph(path: impl AsRef<Path>) -> anyhow::Result<DependencyGraph> {
    let g =  DependencyGraph::load_from_rsf_file(path)?;
    Ok(g)
}

pub fn load_graph_from_file(path: impl AsRef<Path>) -> anyhow::Result<DependencyGraph> {
    let ext = path.as_ref().extension()
        .ok_or_else(|| anyhow::anyhow!("Need a file extension"))?;
    match ext.to_str().expect("Failed to convert file extension to string") {
        "rsf" => load_rsf_graph(path),
        "odem" => load_odem_graph(path),
        x => Err(anyhow::anyhow!("Unknown file extension: {}", x))
    }
}
