//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::collections::HashMap;

use crate::graphs::{DependencyGraph, DependencySpec};
use crate::graphs::loaders::load_graph_from_file;
use crate::utils::paths::ExtractFileName;
use crate::utils::versions::ExtractProjectInformation;
//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Types
//////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionTriple {
    project: String,
    version_1: String,
    version_2: String,
    version_3: String,
    training_graph: Graph,
    test_graph: Graph,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Graph {
    nodes: Vec<Node>,                               // List of all nodes in the graph.
                                                    // Every node knows which features 
                                                    // files (from classes) are associated with it.
    edges: Vec<Edge>,                               // (from, to, type) -- indexes into nodes
    hierarchy_root: usize,                          // The root of the hierarchy.
                                                    // Indexes into hierarchy, and thus nodes.
    edge_labels: HashMap<(usize, usize), bool>,     // Mapping of edge indexes to labels.
                                                    // Indexes into nodes.
                                                    // Generally a subset of edges,
                                                    // but not necessarily. 
    directed: bool,                                 // Whether the graph is directed.
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    name: String,
    children: Vec<usize>,
    feature_files: Vec<String>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    from: usize,
    to: usize,
    edge_type: DependencySpec
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Implementations
//////////////////////////////////////////////////////////////////////////////////////////////////

impl VersionTriple {
    pub fn from_files(path_v1: impl AsRef<std::path::Path>,
                      path_v2: impl AsRef<std::path::Path>,
                      path_v3: impl AsRef<std::path::Path>) -> anyhow::Result<Self>
    {
        let project = path_v1.as_ref().extract_project()?.to_string();
        let version_1 = path_v1.as_ref().extract_version()?.to_string();
        let version_2 = path_v2.as_ref().extract_version()?.to_string();
        let version_3 = path_v3.as_ref().extract_version()?.to_string();
        
        let v1 = load_graph_from_file(path_v1)?;
        let v2 = load_graph_from_file(path_v2)?;
        let v3 = load_graph_from_file(path_v3)?;
        
        let graph = Self {
            project,
            version_1,
            version_2,
            version_3,
            training_graph: Self::build_training_graph(&v1, &v2),
            test_graph: Self::build_test_graph(&v2, &v3),
        };
        Ok(graph)
    }
    
    fn build_training_graph(v1: &DependencyGraph, v2: &DependencyGraph) -> Graph {
        todo!()
    }
    
    fn build_test_graph(v2: &DependencyGraph, v3: &DependencyGraph) -> Graph {
        todo!()
    }
}