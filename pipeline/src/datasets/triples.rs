//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

use itertools::Itertools;

use crate::graphs::{ClassGraph, DependencyGraph, DependencySpec, ModuleGraph};
use crate::graphs::hierarchy::Hierarchy;
use crate::graphs::loaders::load_graph_from_file;
use crate::languages::Language;
use crate::languages::mappers::ObjectLocation;
use crate::utils::versions::ExtractProjectInformation;

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Magic Number
//////////////////////////////////////////////////////////////////////////////////////////////////

const MAGIC_NUMBER: u32 = 0x00_01_01_01;

const V1: u8 = 1;
const V2: u8 = 2;
#[allow(unused)]
const V3: u8 = 3;

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
    pub only_common_nodes_for_training: bool,
    pub magic_number: u32,
    pub gnn_safe: bool,
    pub language: Language
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
    classes: Vec<Class>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Class {
    package: String,
    name: String,
    versions: Vec<u8>
}



#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    name: String,
    versions: Vec<u8>,
    files: HashMap<String, ObjectLocation>
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
    children: Vec<NodeHierarchy>,
    versions: Vec<u8>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeLabels {
    // separated into a struct because of JSON limitations
    edges: Vec<(usize, usize)>,
    labels: Vec<bool>
}

enum NodeOwnership<'a> {
    ExactVersion(u8),
    SplitVersions{
        v1: u8,
        v2: u8,
        v1_nodes: &'a HashSet<String>,
        v2_nodes: &'a HashSet<String>
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Getters/Setters
//////////////////////////////////////////////////////////////////////////////////////////////////

#[allow(unused)]
impl Graph {
    pub fn nodes(&self) -> &Vec<Node> {
        &self.nodes
    }
    pub fn nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
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
    pub fn classes(&self) -> &Vec<Class> {
        &self.classes
    }
}

#[allow(unused)]
impl Class {
    pub fn package(&self) -> &str {
        &self.package
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn versions(&self) -> &Vec<u8> {
        &self.versions
    }
}

#[allow(unused)]
impl Node {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn versions(&self) -> &Vec<u8> {
        &self.versions
    }
    pub fn files(&self) -> &HashMap<String, ObjectLocation> {
        &self.files
    }
    pub fn files_mut(&mut self) -> &mut HashMap<String, ObjectLocation> {
        &mut self.files
    }
}

#[allow(unused)]
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

#[allow(unused)]
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
    pub fn versions(&self) -> &Vec<u8> {
        &self.versions
    }
}

#[allow(unused)]
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
    pub fn training_graph_mut(&mut self) -> &mut Graph {
        &mut self.training_graph
    }
    pub fn test_graph_mut(&mut self) -> &mut Graph {
        &mut self.test_graph
    }

    pub fn from_files(path_v1: impl AsRef<std::path::Path>,
                      path_v2: impl AsRef<std::path::Path>,
                      path_v3: impl AsRef<std::path::Path>,
                      only_common_nodes_for_training: bool,
                      mapping: &HashMap<String, String>,
                      language: Language) -> anyhow::Result<Self>
    {
        let project = path_v1.as_ref().extract_project()?.to_string();
        let project = match mapping.get(&project) {
            Some(p) => p.to_string(),
            None => project
        };
        let version_1 = path_v1.as_ref().extract_version()?.to_string();
        let version_2 = path_v2.as_ref().extract_version()?.to_string();
        let version_3 = path_v3.as_ref().extract_version()?.to_string();

        let v1_cls = load_graph_from_file(path_v1)?;
        let v2_cls = load_graph_from_file(path_v2)?;
        let v3_cls = load_graph_from_file(path_v3)?;

        let v1 = v1_cls.to_module_graph();
        let v2 = v2_cls.to_module_graph();
        let v3 = v3_cls.to_module_graph();

        let triple = Self {
            project,
            version_1,
            version_2,
            version_3,
            training_graph: Self::build_training_graph(&v1, &v2, &v1_cls, &v2_cls, only_common_nodes_for_training),
            test_graph: Self::build_test_graph(&v2, &v3, &v2_cls, &v3_cls),
            metadata: VersionTripleMetadata {
                only_common_nodes_for_training,
                magic_number: MAGIC_NUMBER,
                gnn_safe: only_common_nodes_for_training,
                language
            }
        };
        Ok(triple)
    }

    fn build_training_graph(v1: &DependencyGraph<ModuleGraph>,
                            v2: &DependencyGraph<ModuleGraph>,
                            v1_cls: &DependencyGraph<ClassGraph>,
                            v2_cls: &DependencyGraph<ClassGraph>,
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
            Self::build_training_graph_from_v1(v1, v2, v1_cls, v2_cls)
        } else {
            Self::build_training_graph_from_v1_and_v2(v1, v2, v1_cls, v2_cls)
        }
    }

    fn build_training_graph_from_v1(v1: &DependencyGraph<ModuleGraph>,
                                    v2: &DependencyGraph<ModuleGraph>,
                                    v1_cls: &DependencyGraph<ClassGraph>,
                                    _v2_cls: &DependencyGraph<ClassGraph>) -> Graph {
        log::debug!("Building training graph from v1");
        log::debug!("{:?}", v1.vertices());
        let nodes = Self::nodes_to_index_map(v1.vertices());
        let edges = Self::edge_to_index_list(v1.edges(), &nodes);

        let v2_nodes = v2.vertices() & v1.vertices();
        let test_edges = Self::compute_test_edges(v2_nodes, &nodes, v2.edges());
        let hierarchies = Self::compute_hierarchy(v1, &nodes, V1);
        let nodes = Self::node_map_to_vec(nodes, NodeOwnership::ExactVersion(V1));
        let classes = Self::make_class_map(v1_cls, V1);
        let classes = Self::class_map_to_vec(classes);
        Graph { nodes, edges, hierarchies, edge_labels: test_edges, directed: true, classes }
    }

    fn build_training_graph_from_v1_and_v2(v1: &DependencyGraph<ModuleGraph>,
                                           v2: &DependencyGraph<ModuleGraph>,
                                           v1_cls: &DependencyGraph<ClassGraph>,
                                           v2_cls: &DependencyGraph<ClassGraph>) -> Graph {
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
        let hierarchy_1 = Self::compute_hierarchy(v1, &nodes, V1);
        let hierarchy_2 = Self::compute_hierarchy(v2, &nodes, V2);
        let hierarchies = Self::merge_hierarchies(hierarchy_1, hierarchy_2);

        // Convert nodes
        let nodes = Self::node_map_to_vec(
            nodes,
            NodeOwnership::SplitVersions {
                v1: V1, v2: V2, v1_nodes: &v1.vertices(), v2_nodes: &v2.vertices()
            }
        );
        // Classes
        let classes = Self::merge_class_maps(
            Self::make_class_map(v1_cls, V1),
            Self::make_class_map(v2_cls, V2)
        );
        let classes = Self::class_map_to_vec(classes);

        Graph { nodes, edges, hierarchies, edge_labels: test_edges, directed: true, classes }
    }

    fn build_test_graph(v2: &DependencyGraph<ModuleGraph>,
                        v3: &DependencyGraph<ModuleGraph>,
                        v2_cls: &DependencyGraph<ClassGraph>,
                        _v3_cls: &DependencyGraph<ClassGraph>) -> Graph {
        log::debug!("Building test graph");
        // Generate a graph such that
        //  - Its nodes are those from v2
        //  - Its edges are those from v2
        //  - Use the same hierarchy as v2
        //  - Its labels are the edges from v3, whose nodes are in v2

        let nodes = Self::nodes_to_index_map(v2.vertices());
        let edges = Self::edge_to_index_list(v2.edges(), &nodes);
        //let joint_nodes = v2.vertices() & v3.vertices();
        let edge_labels = Self::compute_test_edges(
            v2.vertices().clone(), &nodes, v3.edges()
        );
        let hierarchies = Self::compute_hierarchy(v2, &nodes, V2);
        let nodes = Self::node_map_to_vec(nodes, NodeOwnership::ExactVersion(V2));
        let classes = Self::make_class_map(v2_cls, V2);
        let classes = Self::class_map_to_vec(classes);
        Graph { nodes, edges, hierarchies, edge_labels, directed: true, classes }
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
                         node_map: &HashMap<&String, usize>,
                         version: u8) -> Vec<NodeHierarchy>
    {
        let mut result = Vec::new();
        let raw_hierarchies: Vec<Hierarchy> = g.into();
        for hierarchy in raw_hierarchies {
            result.push(Self::compute_hierarchy_recursive(hierarchy, node_map, version));
        }
        result
    }

    fn compute_hierarchy_recursive(hierarchy: Hierarchy,
                                   node_map: &HashMap<&String, usize>,
                                   version: u8) -> NodeHierarchy
    {
        let mut children = Vec::new();
        for child in hierarchy.children {
            children.push(Self::compute_hierarchy_recursive(child, node_map, version));
        }
        NodeHierarchy {
            name: hierarchy.name.clone(),
            index: node_map.get(&hierarchy.name).copied(),
            children,
            versions: vec![version]
        }
    }

    fn node_map_to_vec(node_map: HashMap<&String, usize>,
                       node_ownership: NodeOwnership) -> Vec<Node>
    {
        node_map.into_iter()
            .map(|(name, index)| {
                let vertex = Node {
                    name: name.clone(),
                    files: HashMap::new(),
                    versions: match node_ownership {
                        NodeOwnership::ExactVersion(v) => vec![v],
                        NodeOwnership::SplitVersions { v1, v2, v1_nodes, v2_nodes } => {
                            match (v1_nodes.contains(name), v2_nodes.contains(name)) {
                                (true, true) => vec![v1, v2],
                                (true, false) => vec![v1],
                                (false, true) => vec![v2],
                                (false, false) => unreachable!("Node not contained in v1 nor v2")
                            }
                        }
                    }
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
                hierarchies[i].versions.extend(hierarchy.versions.iter().copied());
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

    fn make_class_map(g: &DependencyGraph<ClassGraph>, version: u8) -> HashMap<String, Class> {
        let mut result = HashMap::new();
        for node in g.vertices() {
            let (package, name) = node.rsplit_once('.').unwrap();
            let class = Class {
                package: package.to_string(),
                name: name.to_string(),
                versions: vec![version]
            };
            result.insert(node.clone(), class);
        }
        result
    }

    fn merge_class_maps(mut main: HashMap<String, Class>,
                        new: HashMap<String, Class>) -> HashMap<String, Class>
    {
        for (key, value) in new {
            match main.entry(key) {
                Entry::Occupied(mut e) => {
                    e.get_mut().versions.extend(value.versions.iter().copied());
                }
                Entry::Vacant(e) => {
                    e.insert(value);
                }
            }
        }
        main
    }

    fn class_map_to_vec(class_map: HashMap<String, Class>) -> Vec<Class> {
        class_map.into_values()
            .collect::<Vec<_>>()
    }
}
