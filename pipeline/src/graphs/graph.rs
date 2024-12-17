use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

mod seal {
    pub trait DependencyGraphKind {}
    
    impl DependencyGraphKind for super::ClassGraph {}
    impl DependencyGraphKind for super::ModuleGraph {}
}

pub use seal::DependencyGraphKind;
#[derive(Debug)]
pub struct ClassGraph;
pub struct ModuleGraph;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyGraph<K: DependencyGraphKind> {
    nodes: HashSet<String>,
    edges: HashMap<(String, String), DependencySpec>,
    _kind: std::marker::PhantomData<K>,
}

impl<K: DependencyGraphKind> DependencyGraph<K> {
    pub(super) fn new(nodes: HashSet<String>, 
                      edges: HashMap<(String, String), DependencySpec>) -> Self 
    {
        Self { nodes, edges, _kind: std::marker::PhantomData }
    }
    
    pub fn vertices(&self) -> &HashSet<String> {
        &self.nodes
    }
    
    pub fn edges(&self) -> &HashMap<(String, String), DependencySpec> {
        &self.edges
    }
}

impl DependencyGraph<ClassGraph> {
    pub fn to_module_graph(&self) -> DependencyGraph<ModuleGraph> {
        let mut nodes = HashSet::new();
        let mut edges: HashMap<(String, String), DependencySpec> = HashMap::new();
        for node in self.vertices() {
            let (module, _) = node.rsplit_once('.')
                .unwrap_or_else(|| panic!("Failed to get package from node name: {}", node));
            nodes.insert(module.to_string());
        }
        for ((from, to), spec) in self.edges.iter() {
            let (from_module, _) = from.rsplit_once('.')
                .unwrap_or_else(|| panic!("Failed to get package from node name: {}", from));
            let (to_module, _) = to.rsplit_once('.')
                .unwrap_or_else(|| panic!("Failed to get package from node name: {}", to));
            let key = (from_module.to_string(), to_module.to_string());
            match edges.entry(key) {
                Entry::Occupied(mut e) => {
                    e.get_mut().update_by_ref(spec);
                },
                Entry::Vacant(e) => {
                    e.insert(spec.clone());
                }
            }
        }
        DependencyGraph {
            nodes,
            edges,
            _kind: std::marker::PhantomData,
        }
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
    
    pub fn update_by(&mut self, other: Self) {
        for (key, value) in other.counts {
            *self.counts.entry(key).or_insert(0) += value;
        }
    }

    pub fn update_by_ref(&mut self, other: &Self) {
        for (key, value) in other.counts.iter() {
            *self.counts.entry(*key).or_insert(0) += *value;
        }
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

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Uses => write!(f, "uses"),
            DependencyType::Extends => write!(f, "extends"),
            DependencyType::Implements => write!(f, "implements"),
            DependencyType::Unspecified => write!(f, "unspecified")
        }
    }
}
