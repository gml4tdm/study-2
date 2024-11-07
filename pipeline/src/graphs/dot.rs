use std::collections::HashMap;
use crate::graphs::{DependencyGraph, DependencyGraphKind};

impl<K: DependencyGraphKind> From<&DependencyGraph<K>> for petgraph::Graph<String, String> {
    fn from(value: &DependencyGraph<K>) -> Self {
        let mut graph = Self::new();

        let mut mapping = HashMap::new();
        for vertex in value.vertices() {
            mapping.insert(vertex.clone(), graph.add_node(vertex.to_string()));
        }

        for ((from, to), spec) in value.edges() {
            let from = *mapping.get(from).unwrap();
            let to = *mapping.get(to).unwrap();
            let label = spec.edges()
                .iter()
                .map(|(edge_type, count)| format!("{} ({})", edge_type, count))
                .collect::<Vec<_>>().join(", ");
            graph.add_edge(from, to, label);
        }

        graph
    }
}


impl<K: DependencyGraphKind> DependencyGraph<K> {
    pub fn to_dot(&self) -> String {
        let g: petgraph::Graph<String, String> = self.into();
        let graph = petgraph::dot::Dot::new(&g);
        format!("{:?}", graph)
    }
}
