use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyGraph {
    nodes: HashSet<String>,
    edges: HashMap<(String, String), DependencySpec>,
}

impl DependencyGraph {
    pub(super) fn new(nodes: HashSet<String>, 
                      edges: HashMap<(String, String), DependencySpec>) -> Self 
    {
        Self { nodes, edges }
    }
    
    pub fn vertices(&self) -> &HashSet<String> {
        &self.nodes
    }
    
    pub fn edges(&self) -> &HashMap<(String, String), DependencySpec> {
        &self.edges
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DependencySpec {
    counts: HashMap<DependencyType, usize>,
}

impl DependencySpec {
    pub(super) fn increment(&mut self, dependency_type: DependencyType) {
        *self.counts.entry(dependency_type).or_insert(0) += 1;
    }
    
    pub fn edges(&self) -> &HashMap<DependencyType, usize> {
        &self.counts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum DependencyType {
    Uses,
    Extends,
    Implements,
    Unspecified
}
