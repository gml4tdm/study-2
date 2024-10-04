use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::OnceLock;
use crate::errors::GraphError;
use crate::graph::adjacency::{AdjacencyMatrix, GraphMatrix};

pub struct Graph<T> {
    nodes: HashMap<T, usize>,
    nodes_reversed: Vec<T>,
    edges: HashSet<(usize, usize)>,
    adj: AdjacencyMatrix,
    katz: OnceLock<GraphMatrix<f64>>,
    sim_rank: OnceLock<GraphMatrix<f64>>
}


impl<T: Eq + Hash + Debug + Clone> Graph<T> {
    pub fn new(spec: HashMap<T, HashSet<T>>) -> Self {
        let mut nodes = HashMap::new();
        let mut nodes_reversed = Vec::new();
        let mut edges = HashSet::new();
        
        for (from, tos) in spec {
            let from_index = match nodes.entry(from) {
                std::collections::hash_map::Entry::Occupied(o) => *o.get(),
                std::collections::hash_map::Entry::Vacant(v) => {
                    let index = nodes_reversed.len();
                    nodes_reversed.push(v.key().clone());
                    v.insert(index);
                    index
                }
            };
            for to in tos {
                let to_index = match nodes.entry(to) {
                    std::collections::hash_map::Entry::Occupied(o) => *o.get(),
                    std::collections::hash_map::Entry::Vacant(v) => {
                        let index = nodes_reversed.len();
                        nodes_reversed.push(v.key().clone());
                        v.insert(index);
                        index
                    }
                };
                edges.insert((from_index, to_index));
            }
        }
        
        let mut adj = AdjacencyMatrix::new(nodes_reversed.len());
        for (from, to) in &edges {
            adj.connect(*from, *to);
        }
        
        Graph { nodes, nodes_reversed, edges, adj, katz: OnceLock::new(), sim_rank: OnceLock::new() }
    }
    
    pub fn nodes(&self) -> &[T] {
        self.nodes_reversed.as_slice()
    }
}

impl<T: Eq + Hash + Debug> Graph<T> {
    fn get_vertex_index(&self, vertex: &T) -> Result<usize, GraphError> {
        self.nodes.get(vertex)
            .ok_or(GraphError::UndefinedVertex{vertex: format!("{:?}", vertex)})
            .map(|i| *i)
    }
    
    pub fn n_common_neighbours(&self, a: &T, b: &T) -> Result<i32, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        Ok(self.adj.common_neighbour_count(a_index, b_index))
    }
    
    pub fn salton_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        Ok(self.adj.salton_metric(a_index, b_index))
    }
    
    pub fn sorenson_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        Ok(self.adj.sorensen_metric(a_index, b_index))
    }
    
    pub fn adamic_adar_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        Ok(self.adj.adamic_adar_metric(a_index, b_index))
    }
    
    pub fn russel_rao_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        Ok(self.adj.russel_rao_metric(a_index, b_index))
    }
    
    pub fn resource_allocation_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        Ok(self.adj.resource_allocation_metric(a_index, b_index))
    }
    
    pub fn katz_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        let matrix = self.katz.get_or_init(|| self.adj.katz_metric());
        Ok(matrix.score(a_index, b_index))
    }
    
    pub fn sim_rank_metric(&self, a: &T, b: &T) -> Result<f64, GraphError> {
        let a_index = self.get_vertex_index(a)?;
        let b_index = self.get_vertex_index(b)?;
        let matrix = self.sim_rank.get_or_init(|| self.adj.sim_rank_metric());
        Ok(matrix.score(a_index, b_index))
    }
}
