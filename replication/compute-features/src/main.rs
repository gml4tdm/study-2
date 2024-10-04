use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use nalgebra::DimAdd;
use crate::csv_schema::Record;
use crate::output_schema::{Edge, GraphFeatureData, LinkFeature};
use crate::xml_schema::DependencyGraph;

mod xml_schema;
mod graph;
mod errors;
mod csv_schema;
mod output_schema;

const PATTERN: OnceCell<regex::Regex> = OnceCell::new();


fn collect_workload_files(data_dir: PathBuf) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    let mut workload = Vec::new();
    for result in std::fs::read_dir(data_dir.as_path())? {
        let entry = result?;
        if !entry.metadata()?.is_dir() {
            continue;
        }
        let dir_path = data_dir.join(entry.file_name());

        for inner_result in std::fs::read_dir(dir_path.as_path())? {
            let inner_entry = inner_result?;
            if !inner_entry.metadata()?.is_file() {
                continue;
            }
            let filename = inner_entry.file_name()
                .to_str()
                .expect("Invalid Filename")
                .to_string();
            if !filename.ends_with(".odem") {
                continue;
            }
            let semantic_file = find_semantic_file(&dir_path, &filename)?;
            workload.push(
                (dir_path.join(inner_entry.file_name()), dir_path.join(semantic_file))
            );
        }
    }
    Ok(workload)
}


fn find_semantic_file(path: &PathBuf, filename: &str) -> anyhow::Result<String> {
    let prefix = PATTERN
        .get_or_init(
            || regex::Regex::new(r"[a-zA-Z_\-0-9]+-\d+(\.\d+)*").expect("Invalid Regex"))
        .find_at(filename, 0)
        .ok_or_else(|| anyhow::anyhow!(format!("Invalid Filename: {}", filename)))
        .map(|inner| inner.as_str().to_string())?;
    for result in std::fs::read_dir(path)? {
        let entry = result?;
        if !entry.metadata()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_str().expect("Invalid Filename").to_string();
        if name.starts_with(&prefix) && name.ends_with(".txt") {
            return Ok(name);
        }
    }
    Err(anyhow::anyhow!(format!("Semantic File Not Found: {}", filename)))
}

fn handle_file_pair(graph_file: PathBuf, semantic_file: PathBuf) -> anyhow::Result<GraphFeatureData> {
    let graph = build_graph_from_file(graph_file)?;
    let similarities = load_csv_data(semantic_file)?;
    let mut sim_by_key = similarities.into_iter()
        .map(|r| ((r.from.clone(), r.to.clone()), r))
        .collect::<HashMap<_, _>>();
    let mut nodes: HashSet<String> = HashSet::new();
    let mut edges = HashSet::new();
    let mut no_semantic = HashSet::new();
    let mut no_graph = HashSet::new();
    let mut features = Vec::new();
    for x in graph.nodes() {
        let z = x.clone();
        nodes.insert(z);
        for y in graph.nodes() {
            if x == y {
                continue;
            }
            let key = (x.clone(), y.clone());
            edges.insert(key.clone());
            match sim_by_key.entry(key.clone()) {
                Entry::Occupied(e) => {
                    let semantics = e.remove();
                    let ld = LinkFeature {
                        edge: Edge {
                            from:  x.clone(),
                            to: y.clone(),
                        },
                        common_neighbours: graph.n_common_neighbours(x, y)?,
                        salton: graph.salton_metric(x, y)?,
                        sorenson: graph.sorenson_metric(x, y)?,
                        adamic_adar: graph.adamic_adar_metric(x, y)?,
                        russel_rao: graph.russel_rao_metric(x, y)?,
                        resource_allocation: graph.resource_allocation_metric(x, y)?,
                        katz: graph.katz_metric(x, y)?,
                        sim_rank: graph.sim_rank_metric(x, y)?,
                        cosine_1: semantics.cosine_1,
                        cosine_2: semantics.cosine_2,
                        // up to 16
                        cosine_3: semantics.cosine_3,
                        cosine_4: semantics.cosine_4,
                        cosine_5: semantics.cosine_5,
                        cosine_6: semantics.cosine_6,
                        cosine_7: semantics.cosine_7,
                        cosine_8: semantics.cosine_8,
                        cosine_9: semantics.cosine_9,
                        cosine_10: semantics.cosine_10,
                        cosine_11: semantics.cosine_11,
                        cosine_12: semantics.cosine_12,
                        cosine_13: semantics.cosine_13,
                        cosine_14: semantics.cosine_14,
                        cosine_15: semantics.cosine_15,
                        cosine_16: semantics.cosine_16,
                    };
                    features.push(ld);
                }
                Entry::Vacant(_) => {
                    no_semantic.insert(key);
                }
            }
        }
    }
    for ((x, y), _) in sim_by_key {
        no_graph.insert((x, y));
    }
    
    let final_data = GraphFeatureData {
        nodes: nodes.into_iter().collect(),
        edges: edges.into_iter()
            .map(|(from, to)| Edge { from, to } )
            .collect(),
        pairs_without_semantic_features: no_semantic.into_iter()
            .map(|(from, to)| Edge { from, to } )
            .collect(),
        pairs_without_topological_features: no_graph.into_iter()
            .map(|(from, to)| Edge { from, to })
            .collect(),
        link_features: features
    };
    
    Ok(final_data)
}

fn load_csv_data(filename: PathBuf) -> anyhow::Result<Vec<Record>> {
    let mut reader = csv::Reader::from_path(filename)?;
    let mut results = Vec::new();
    for result in reader.deserialize() {
        let record: Record = result?;
        results.push(record);
    }
    Ok(results)
}

fn build_graph_from_file(graph_file: PathBuf) -> anyhow::Result<graph::Graph<String>> {
    let f =std::fs::File::open(graph_file)?;
    let buf = std::io::BufReader::new(f);
    let xml = quick_xml::de::from_reader::<_, DependencyGraph>(buf)?;
    let mut builder = graph::GraphBuilder::new();
    let containers = xml.context.containers;
    if containers.len() != 1 {
        return Err(anyhow::anyhow!("Invalid Container Count: {}", containers.len()));
    }
    let container = containers.into_iter().nth(1).unwrap();
    for namespace in container.namespaces {
        let ns_name = namespace.name;
        builder.add_vertex_in_place(ns_name.clone())?;
        for r#type in namespace.types {
            if r#type.dependencies.count > 0 {
                for dep in r#type.dependencies.dependencies {
                    let dep_name = dep.name;
                    // Split name on dots and join the first n-1 components using dots
                    let dep_parts: Vec<&str> = dep_name.split('.').collect();
                    let dep_ns = dep_parts[..dep_parts.len() - 1].join(".");
                    // ignore error because we don't care
                    let _ = builder.add_vertex_in_place(dep_name.clone());
                    let _ = builder.add_edge_in_place(ns_name.clone(), dep_ns.clone());
                }
            }
        }
    }

    Ok(builder.build())
}


fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <directory>", args[0]);
        return Ok(());
    }
    let data_dir = PathBuf::from(&args[1]);
    let workload = collect_workload_files(data_dir)?;

    for (graph_file, semantic_file) in workload {
        println!("Processing {} and {}", graph_file.display(), semantic_file.display());
        
        let features = handle_file_pair(graph_file.clone(), semantic_file)?;
    
        let out_file = graph_file.with_extension("json");
        let f = std::fs::File::create(out_file)?;
        serde_json::to_writer_pretty(f, &features)?;
    }

    Ok(())
}
