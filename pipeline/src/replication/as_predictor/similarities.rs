use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use crate::graphs::{DependencyGraph, ModuleGraph};
use crate::utils::rsf::read_rsf_file;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotatedEdge {
    pub from: String,
    pub to: String,
    pub features: HashMap<String, f64>,
    pub present_in_graph: bool,
}


pub fn build_edge_list(graph: &DependencyGraph<ModuleGraph>,
                       similarity_file: PathBuf) -> anyhow::Result<Vec<AnnotatedEdge>>
{
    let graph_edges = graph.edges()
        .keys()
        .map(|x| (x.0.clone(), x.1.clone()))
        .collect::<HashSet<_>>();
    
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for (edge, features) in aggregate_similarities(similarity_file)? {
        let present_in_graph = graph_edges.contains(&edge);
        seen.insert(edge.clone());
        let (from, to) = edge;
        result.push(AnnotatedEdge {
            from,
            to,
            features,
            present_in_graph
        });
    }
    
    let missing = graph_edges.difference(&seen).collect::<Vec<_>>();
    log::warn!("Missing edges from similarity file: {:?}", missing.len());
    
    Ok(result)
}

fn aggregate_similarities(path: PathBuf) -> anyhow::Result<HashMap<(String, String), HashMap<String, f64>>> {
    let mut result = HashMap::new();
    for similarity in parse_similarities(path)? {
        let key = (similarity.from, similarity.to);
        let entry = result.entry(key).or_insert(HashMap::new());
        entry.insert(similarity.metric, similarity.value);
    }
    Ok(result)
}

struct EdgeDescriptor {
    from: String,
    to: String
}

impl FromStr for EdgeDescriptor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // trim leading and trailing quote 
        let s = s.strip_prefix('"').unwrap_or(s);
        let s = s.strip_suffix('"').unwrap_or(s);
        // trim brackets
        let s = s.strip_prefix('(')
            .ok_or_else(|| anyhow::anyhow!("Invalid Edge"))?;
        let s = s.strip_suffix(')')
            .ok_or_else(|| anyhow::anyhow!("Invalid Edge"))?;
        // split by comma
        let parts = s.split(',').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!("Invalid Edge"));
        }
        // Check first entry 
        if parts[0] != "\"dependency\"" {
            panic!("Unknown type in similarity file: {}", parts[0]);
        }
        let source = parts[1].strip_prefix('"').unwrap_or(parts[1]);
        let source = source.strip_suffix('"').unwrap_or(source);
        let target = parts[2].strip_prefix('"').unwrap_or(parts[2]);
        let target = target.strip_suffix('"').unwrap_or(target);
        Ok(EdgeDescriptor {
            from: source.to_string(),
            to: target.to_string()
        })
    }
}

struct Similarity {
    metric: String,
    from: String,
    to: String,
    value: f64,
}

impl From<(String, EdgeDescriptor, f64)> for Similarity {
    fn from((metric, edge, value): (String, EdgeDescriptor, f64)) -> Self {
        Similarity {
            metric,
            from: edge.from,
            to: edge.to,
            value
        }
    }
}


fn parse_similarities(path: PathBuf) -> anyhow::Result<Vec<Similarity>> {
    read_rsf_file::<Similarity, String, EdgeDescriptor, f64, _, _, _>(path)
}
    