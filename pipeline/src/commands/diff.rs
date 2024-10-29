use std::path::PathBuf;


use crate::graphs::loaders::rsf::FromRsfFile;
use crate::graphs::DependencyGraph;
use crate::graphs::diff::diff_graphs;
use crate::graphs::loaders::odem::OdemGraphRoot;

pub fn diff_graph_commnd(old: PathBuf, new: PathBuf) -> anyhow::Result<()> {
    let lhs = load_graph_from_file(old)?;
    let rhs = load_graph_from_file(new)?;
    let diff = diff_graphs(&lhs, &rhs);
    println!("{}", diff.format_diff());
    Ok(())
}


fn load_graph_from_file(path: PathBuf) -> anyhow::Result<DependencyGraph> {
    let ext = path.extension()
        .ok_or_else(|| anyhow::anyhow!("Need a file extension"))?;
    match ext.to_str().expect("Failed to convert file extension to string") {
        "rsf" => {
            let g =  DependencyGraph::load_from_rsf_file(path)?;
            Ok(g)
        },
        "odem" => { 
            let odem = OdemGraphRoot::load_from_file(path)?;
            Ok(DependencyGraph::from(odem))
        },
        x => Err(anyhow::anyhow!("Unknown file extension: {}", x))
    }
}
