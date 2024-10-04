use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::hash::Hash;
use crate::errors;


pub struct GraphBuilder<T> {
    graph: HashMap<T, HashSet<T>>,
}

impl<T> GraphBuilder<T> {
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
        }
    }
}

impl<T: Eq + Hash + Debug + Clone> GraphBuilder<T> {
    pub fn add_vertex_in_place(&mut self, vertex: T) -> Result<(), errors::GraphBuilderError> {
        match self.graph.entry(vertex) {
            Entry::Occupied(e) => {
                Err(errors::GraphBuilderError::DuplicateVertex{
                    vertex: format!("{:?}", e.key())
                })
            }
            Entry::Vacant(e) => {
                e.insert(HashSet::new());
                Ok(())
            }
        }
    }

    pub fn add_edge_in_place(&mut self, from: T, to: T) -> Result<(), errors::GraphBuilderError> {
        match self.graph.get_mut(&from) {
            None => Err(errors::GraphBuilderError::UndefinedVertex{
                vertex: format!("{:?}", from)
            }),
            Some(targets) => {
                if targets.contains(&to) {
                    Err(errors::GraphBuilderError::DuplicateEdge{
                        from_vertex: format!("{:?}", from),
                        to_vertex: format!("{:?}", to)
                    })
                } else {
                    targets.insert(to);
                    Ok(())
                }
            }
        }
    }

    pub fn build(self) -> super::core::Graph<T> {
        super::core::Graph::new(self.graph)
    }
}
