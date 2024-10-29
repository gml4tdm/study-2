use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use crate::graphs::{DependencyGraph, DependencySpec, DependencyType};
use crate::utils::rsf::read_rsf_file;

#[allow(unused)]
pub trait FromRsfFile: Sized {
    fn load_from_rsf_file(path: impl AsRef<Path>) -> anyhow::Result<Self>;
}

struct DependencyEdge {
    from: String,
    to: String
}

impl FromStr for DependencyEdge {
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
        if parts[0] != "\"depends\"" {
            panic!("Unknown dependency type for RSF graph: {}", parts[0]);
        }
        let source = parts[1].strip_prefix('"').unwrap_or(parts[1]);
        let source = source.strip_suffix('"').unwrap_or(source);
        let target = parts[2].strip_prefix('"').unwrap_or(parts[2]);
        let target = target.strip_suffix('"').unwrap_or(target);
        Ok(DependencyEdge {
            from: source.to_string(),
            to: target.to_string()
        })
    }
}

struct Header;

impl FromStr for Header {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s != "dependency" {
            return Err(anyhow::anyhow!("Invalid dependency line header"));
        }
        Ok(Header)
    }
}

struct Dependency {
    edge: DependencyEdge,
    #[allow(unused)] count: f32
}

impl From<(Header, DependencyEdge, f32)> for Dependency {
    fn from((_header, edge, count): (Header, DependencyEdge, f32)) -> Self {
        Dependency {
            edge,
            count
        }
    }
}

impl FromRsfFile for DependencyGraph {
    fn load_from_rsf_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw_edges = read_rsf_file::<Dependency, _, _, _, _, _, _>(path)?;
        let mut vertices = HashSet::new();
        let mut edges = HashMap::new();
        for raw in raw_edges {
            vertices.insert(raw.edge.to.clone());
            vertices.insert(raw.edge.from.clone());
            let key = (raw.edge.to, raw.edge.from);
            edges.entry(key).or_insert(DependencySpec::default())
                .increment(DependencyType::Unspecified);
        }
        Ok(DependencyGraph::new(vertices, edges))
    }
}
