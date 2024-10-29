use std::path::PathBuf;

use crate::graphs::diff::diff_graphs;
use crate::graphs::loaders::load_graph_from_file;

pub fn diff_graph_commnd(old: PathBuf, new: PathBuf) -> anyhow::Result<()> {
    let lhs = load_graph_from_file(old)?;
    let rhs = load_graph_from_file(new)?;
    let diff = diff_graphs(&lhs, &rhs);
    println!("{}", diff.format_diff());
    Ok(())
}
