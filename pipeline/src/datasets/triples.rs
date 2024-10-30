#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionTriple {
    name: String,
    version_1: String,
    version_2: String,
    version_3: String,
    training_graph: Graph,
    test_graph: Graph,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Graph {
    nodes: Vec<Node>,               // List of all nodes in the graph.
                                    // Every node knows which features 
                                    // files (from classes) are associated with it.
    edges: Vec<(usize, usize)>,     // (from, to) -- indexes into nodes
    hierarchy: Vec<Vec<usize>>,     // hierarchy[i] is the list of nodes that 
                                    // are children of node i.
                                    // Indexes into nodes.
    hierarchy_root: usize,          // The root of the hierarchy.
                                    // Indexes into hierarchy, and thus nodes.
    directed: bool,                 // Whether the graph is directed.
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    name: String,
    feature_files: Vec<String>
}
