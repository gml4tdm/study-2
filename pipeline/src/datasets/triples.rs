//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

use itertools::Itertools;

use crate::graphs::{DependencyGraph, DependencySpec, ModuleGraph};
use crate::graphs::hierarchy::Hierarchy;
use crate::graphs::loaders::load_graph_from_file;
use crate::utils::versions::ExtractProjectInformation;

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Magic Number 
//////////////////////////////////////////////////////////////////////////////////////////////////

const MAGIC_NUMBER: u32 = 0x00_01_01_01;

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
    metadata: VersionTripleMetadata,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionTripleMetadata {
    only_common_nodes_for_training: bool,
    magic_number: u32,
    gnn_safe: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Graph {
    nodes: Vec<Node>,                               // List of all nodes in the graph.
                                                    // Every node knows which features 
                                                    // files (from classes) are associated with it.
    edges: Vec<Edge>,                               // (from, to, type) -- indexes into nodes
    hierarchies: Vec<NodeHierarchy>,
    edge_labels: EdgeLabels,                        // Mapping of edge indexes to labels.
                                                    // Indexes into nodes.
                                                    // Generally a subset of edges,
                                                    // but not necessarily. 
    directed: bool,                                 // Whether the graph is directed.
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    name: String,
    feature_files: Vec<String>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    from: usize,
    to: usize,
    edge_type: DependencySpec
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeHierarchy {
    name: String,
    index: Option<usize>,
    children: Vec<NodeHierarchy>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeLabels {
    // separated into a struct because of JSON limitations 
    edges: Vec<(usize, usize)>,
    labels: Vec<bool>
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Getters/Setters
//////////////////////////////////////////////////////////////////////////////////////////////////

impl Graph {
    pub fn nodes(&self) -> &Vec<Node> {
        &self.nodes
    }
    pub fn edges(&self) -> &Vec<Edge> {
        &self.edges
    }
    pub fn edge_labels(&self) -> &EdgeLabels {
        &self.edge_labels
    }
    pub fn hierarchies(&self) -> &Vec<NodeHierarchy> {
        &self.hierarchies
    }
    pub fn is_directed(&self) -> bool {
        self.directed
    }
}

impl Node {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn files(&self) -> &Vec<String> {
        &self.feature_files
    }
    pub fn files_mut(&mut self) -> &mut Vec<String> {
        &mut self.feature_files
    }
}

impl Edge {
    pub fn from(&self) -> usize {
        self.from
    }
    pub fn to(&self) -> usize {
        self.to
    }
    pub fn edge_type(&self) -> &DependencySpec {
        &self.edge_type
    }
}

impl NodeHierarchy {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn index(&self) -> Option<usize> {
        self.index
    }
    pub fn children(&self) -> &Vec<NodeHierarchy> {
        &self.children
    }
}

impl EdgeLabels {
    pub fn labels(&self) -> &Vec<bool> {
        &self.labels
    }
    pub fn edges(&self) -> &Vec<(usize, usize)> {
        &self.edges 
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Implementations
//////////////////////////////////////////////////////////////////////////////////////////////////

#[allow(unused)]
impl VersionTriple {
    pub fn project(&self) -> &str {
        &self.project
    }   
    pub fn version_1(&self) -> &str {
        &self.version_1
    }
    pub fn version_2(&self) -> &str {
        &self.version_2
    }
    pub fn version_3(&self) -> &str {
        &self.version_3
    }
    pub fn metadata(&self) -> VersionTripleMetadata {
        self.metadata
    }
    pub fn training_graph(&self) -> &Graph {
        &self.training_graph
    }
    pub fn test_graph(&self) -> &Graph {
        &self.test_graph
    }
    
    pub fn from_files(path_v1: impl AsRef<std::path::Path>,
                      path_v2: impl AsRef<std::path::Path>,
                      path_v3: impl AsRef<std::path::Path>,
                      only_common_nodes_for_training: bool, 
                      mapping: HashMap<String, String>) -> anyhow::Result<Self>
    {
        let project = path_v1.as_ref().extract_project()?.to_string();
        let project = match mapping.get(&project) {
            Some(p) => p.to_string(),
            None => project
        };
        let version_1 = path_v1.as_ref().extract_version()?.to_string();
        let version_2 = path_v2.as_ref().extract_version()?.to_string();
        let version_3 = path_v3.as_ref().extract_version()?.to_string();
        
        let v1 = load_graph_from_file(path_v1)?;
        let v2 = load_graph_from_file(path_v2)?;
        let v3 = load_graph_from_file(path_v3)?;
        
        let v1 = v1.to_module_graph();
        let v2 = v2.to_module_graph();
        let v3 = v3.to_module_graph();
        
        let triple = Self {
            project,
            version_1,
            version_2,
            version_3,
            training_graph: Self::build_training_graph(&v1, &v2, only_common_nodes_for_training),
            test_graph: Self::build_test_graph(&v2, &v3),
            metadata: VersionTripleMetadata {
                only_common_nodes_for_training,
                magic_number: MAGIC_NUMBER,
                gnn_safe: only_common_nodes_for_training
            }
        };
        Ok(triple)
    }
    
    fn build_training_graph(v1: &DependencyGraph<ModuleGraph>,
                            v2: &DependencyGraph<ModuleGraph>,
                            only_common_nodes_for_training: bool) -> Graph
    {
        // Build a graph such that
        //  - if only_common_nodes_for_training is true
        //      - Its nodes are those from v1
        //      - Its edges are those from v1
        //      - Its hierarchy is the same as v1
        //      - Its labels are the edges from v2, whose nodes are in v1
        //  - otherwise
        //      - Its nodes are those from v1 and v2
        //      - Its edges are those from v1 and v2
        //      - Its hierarchy is the same as v1,
        //          except for added nodes.
        //      - Its labels are created from both v1 and v2.
        if only_common_nodes_for_training {
            Self::build_training_graph_from_v1(v1, v2)
        } else {
            Self::build_training_graph_from_v1_and_v2(v1, v2)
        }
    }

    fn build_training_graph_from_v1(v1: &DependencyGraph<ModuleGraph>, 
                                    v2: &DependencyGraph<ModuleGraph>) -> Graph {
        log::debug!("Building training graph from v1");
        log::debug!("{:?}", v1.vertices());
        let nodes = Self::nodes_to_index_map(v1.vertices());
        let edges = Self::edge_to_index_list(v1.edges(), &nodes);

        let v2_nodes = v2.vertices() & v1.vertices();
        let test_edges = Self::compute_test_edges(v2_nodes, &nodes, v2.edges());
        let hierarchies = Self::compute_hierarchy(v1, &nodes);
        let nodes = Self::node_map_to_vec(nodes);
        Graph { nodes, edges, hierarchies, edge_labels: test_edges, directed: true }
    }

    fn build_training_graph_from_v1_and_v2(v1: &DependencyGraph<ModuleGraph>,
                                           v2: &DependencyGraph<ModuleGraph>) -> Graph {
        log::debug!("Building training graph from v1 and v2");
        // Nodes 
        let joint_nodes = v1.vertices() | v2.vertices();
        let nodes = Self::nodes_to_index_map(&joint_nodes);
        // Edges 
        let mut joint_edges = v1.edges().clone();
        for (key, spec) in v2.edges().clone() {
            match joint_edges.entry(key) {
                Entry::Occupied(mut e) => {
                    e.get_mut().update_by(spec);
                }
                Entry::Vacant(e) => {
                    e.insert(spec);
                }
            }
        }
        let edges = Self::edge_to_index_list(&joint_edges, &nodes);
        // Edge labels 
        let mut test_edges = Self::compute_test_edges(
            v1.vertices().clone(), &nodes, v1.edges()
        );
        let more_edges = Self::compute_test_edges(
            v2.vertices().clone(), &nodes, v2.edges()
        );
        test_edges.edges.extend(more_edges.edges);
        test_edges.labels.extend(more_edges.labels);
        // Hierarchy
        let hierarchy_1 = Self::compute_hierarchy(v1, &nodes);
        let hierarchy_2 = Self::compute_hierarchy(v2, &nodes);
        let hierarchies = Self::merge_hierarchies(hierarchy_1, hierarchy_2);
        
        // Convert nodes 
        let nodes = Self::node_map_to_vec(nodes);
        Graph { nodes, edges, hierarchies, edge_labels: test_edges, directed: true }
    }
    
    fn build_test_graph(v2: &DependencyGraph<ModuleGraph>, 
                        v3: &DependencyGraph<ModuleGraph>) -> Graph {
        log::debug!("Building test graph");
        // Generate a graph such that
        //  - Its nodes are those from v2
        //  - Its edges are those from v2
        //  - Use the same hierarchy as v2
        //  - Its labels are the edges from v3, whose nodes are in v2

        let nodes = Self::nodes_to_index_map(v2.vertices());
        let edges = Self::edge_to_index_list(v2.edges(), &nodes);
        let joint_nodes = v2.vertices() & v3.vertices();
        let edge_labels = Self::compute_test_edges(
            joint_nodes, &nodes, v3.edges()
        );
        let hierarchies = Self::compute_hierarchy(v2, &nodes);
        let nodes = Self::node_map_to_vec(nodes);
        Graph { nodes, edges, hierarchies, edge_labels, directed: true }
    }

    fn nodes_to_index_map<'a>(nodes: impl IntoIterator<Item=&'a String>) -> HashMap<&'a String, usize> {
        nodes.into_iter()
            .enumerate()
            .map(|(index, name)| (name, index))
            .collect::<HashMap<_, _>>()
    }

    fn edge_to_index_list<'a, E>(edges: E, node_map: &HashMap<&'a String, usize>) -> Vec<Edge>
    where
        E: IntoIterator<Item=(&'a (String, String), &'a DependencySpec)>,
    {
        edges.into_iter()
            .map(
                |((from, to), spec)| Edge {
                    from: *node_map.get(from)
                        .unwrap_or_else(|| panic!("Node {from} not found in {node_map:?}")),
                    to: *node_map.get(to)
                        .unwrap_or_else(|| panic!("Node {to} not found in {node_map:?}")),
                    edge_type: spec.clone()
                }
            )
            .collect::<Vec<_>>()
    }
    
    fn compute_test_edges(vertices: HashSet<String>, 
                          node_map: &HashMap<&String, usize>,
                          connected: &HashMap<(String, String), DependencySpec>) -> EdgeLabels
    {
        let test_edges = vertices.iter()
            .cartesian_product(vertices.iter())
            .collect::<HashSet<_>>();
        let indices = test_edges.iter()
            .map(|(from, to)| {
                let from_index = *node_map.get(from)
                    .unwrap_or_else(|| panic!("Node {from} not found in {node_map:?}"));
                let to_index = *node_map.get(to)
                    .unwrap_or_else(|| panic!("Node {to} not found in {node_map:?}"));
                (from_index, to_index)
            })
            .collect::<Vec<_>>();
        let labels = test_edges.into_iter()
            .map(|(from, to)| {
                connected.contains_key(&(from.clone(), to.clone()))
            })
            .collect::<Vec<_>>();
        EdgeLabels {
            edges: indices,
            labels
        }
    }
    
    fn compute_hierarchy(g: &DependencyGraph<ModuleGraph>, 
                         node_map: &HashMap<&String, usize>) -> Vec<NodeHierarchy>
    {
        let mut result = Vec::new();
        let raw_hierarchies: Vec<Hierarchy> = g.into();
        for hierarchy in raw_hierarchies {
            result.push(Self::compute_hierarchy_recursive(hierarchy, node_map));
        }
        result
    }
    
    fn compute_hierarchy_recursive(hierarchy: Hierarchy, 
                                   node_map: &HashMap<&String, usize>) -> NodeHierarchy
    {
        let mut children = Vec::new();
        for child in hierarchy.children {
            children.push(Self::compute_hierarchy_recursive(child, node_map));
        }
        NodeHierarchy {
            name: hierarchy.name.clone(),
            index: node_map.get(&hierarchy.name).copied(),
            children
        }
    }
    
    fn node_map_to_vec(node_map: HashMap<&String, usize>) -> Vec<Node> 
    {
        node_map.into_iter()
            .map(|(name, index)| {
                let vertex = Node {
                    name: name.clone(),
                    feature_files: Vec::new()
                };
                (index, vertex)
            })
            .sorted_by_key(|(index, _)| *index)
            .map(|(_, vertex)| vertex)
            .collect::<Vec<_>>()
    }
    
    fn merge_hierarchies(mut main: Vec<NodeHierarchy>, 
                         new: Vec<NodeHierarchy>) -> Vec<NodeHierarchy>
    {
        for hierarchy in new {
            if !Self::merge_hierarchy_recursive(&mut main, &hierarchy) {
                main.push(hierarchy);
            }
        }
        main  
    }
    
    fn merge_hierarchy_recursive(hierarchies: &mut Vec<NodeHierarchy>,
                                 hierarchy: &NodeHierarchy) -> bool
    {
        let mut found = false;
        for i in 0..hierarchies.len() {
            if hierarchies[i].name == hierarchy.name {
                found = true;
                // We found a node match; now recursively merge children 
                for child in hierarchy.children.iter() {
                    if !Self::merge_hierarchy_recursive(&mut hierarchies[i].children, &child) {
                        hierarchies[i].children.push(child.clone());
                    }
                }
                break;
            }
        }
        found
    }
}
