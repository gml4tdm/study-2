use crate::graphs::{DependencyGraph, DependencyGraphKind};
use crate::utils::tree::Tree;
use crate::utils::trie::Trie;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hierarchy {
    pub name: String,
    pub children: Vec<Hierarchy>
}


impl<K: DependencyGraphKind> From<&DependencyGraph<K>> for Vec<Hierarchy> {
    fn from(graph: &DependencyGraph<K>) -> Self {
        let mut trie = Trie::new();
        for vertex in graph.vertices() {
            let parts = vertex.split('.')
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            trie.insert(&parts);
        }
        let trees: Vec<Tree<Vec<String>>> = trie.into();
        trees.into_iter().map(|tree| tree.into()).collect()
    }
}

impl<K: DependencyGraphKind> From<DependencyGraph<K>> for Vec<Hierarchy> {
    fn from(graph: DependencyGraph<K>) -> Self {
        Self::from(&graph)
    }
}

impl From<Tree<Vec<String>>> for Hierarchy {
    fn from(value: Tree<Vec<String>>) -> Self {
        match value {
            Tree::Node { payload, children } => {
                Hierarchy { 
                    name: payload.join("."), 
                    children: children.into_iter().map(Self::from).collect::<Vec<_>>()
                }
            }
            Tree::Leaf { payload } => {
                Hierarchy { 
                    name: payload.join("."), children: Vec::new()
                }
            }
        }
    }
}
