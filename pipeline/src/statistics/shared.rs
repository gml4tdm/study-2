///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
///////////////////////////////////////////////////////////////////////////////////////////////////

use std::collections::{HashMap, HashSet};
use itertools::Itertools;

use crate::graphs::{DependencyGraph, DependencyGraphKind, DependencyType};

///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Auxiliary Statistics 
///////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Copy, Clone, serde::Serialize)]
pub struct Statistics {
    mean: f64,
    median: f64,
    std_dev: f64
}

impl<T: Iterator<Item=f64>> From<T> for Statistics {
    fn from(values: T) -> Self {
        let values = values.collect::<Vec<_>>();
        Self::from_vec(values)
    }
}

impl Statistics {
    fn from_vec(values: Vec<f64>) -> Self {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let ordered = values.iter()
            .copied()
            .sorted_by(ord_float)
            .collect::<Vec<_>>();
        let median = if ordered.len() % 2 == 0 {
            (ordered[ordered.len() / 2 - 1] + ordered[ordered.len() / 2]) / 2.0
        } else {
            ordered[ordered.len() / 2]
        };
        let summed = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
        let variance = summed / values.len() as f64;
        let std_dev = variance.sqrt();
        Self {
            mean,
            median,
            std_dev
        }
    }
}

fn ord_float(a: &f64, b: &f64) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
}

///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Graph Degree Analyser 
///////////////////////////////////////////////////////////////////////////////////////////////////


pub struct GraphDegreeAnalyser {
    incoming: HashMap<String, ConnectionInfo>,
    outgoing: HashMap<String, ConnectionInfo>,
}

#[derive(Debug, Clone, Default)]
struct ConnectionInfo {
    by_type: Vec<(String, DependencyType, u64)>,
}

impl GraphDegreeAnalyser {
    pub fn new<K: DependencyGraphKind>(g: &DependencyGraph<K>) -> Self {
        let mut incoming = HashMap::new();
        let mut outgoing = HashMap::new();
        for ((from, to), spec) in g.edges() {
            for (dependency_type, count) in spec.edges() {
                incoming.entry(to.clone()).or_insert_with(ConnectionInfo::default)
                    .by_type
                    .push((from.clone(), *dependency_type, *count as u64));
                outgoing.entry(from.clone()).or_insert_with(ConnectionInfo::default)
                    .by_type
                    .push((to.clone(), *dependency_type, *count as u64));
            }
        }
        Self { incoming, outgoing }
    }

    // ##### Degree ##### 

    pub fn in_degree(&self) -> Vec<u64> {
        self.incoming.values()
            .map(|c| c.by_type.iter().map(|(_, _, c)| *c).sum::<u64>())
            .collect()
    }

    pub fn out_degree(&self) -> Vec<u64> {
        self.outgoing.values()
            .map(|c| c.by_type.iter().map(|(_, _, c)| *c).sum::<u64>())
            .collect()
    }

    // ##### Degree No Self ##### 

    pub fn in_degree_no_self(&self) -> Vec<u64> {
        Self::degree_no_self(self.incoming.iter())
    }

    pub fn out_degree_no_self(&self) -> Vec<u64> {
        Self::degree_no_self(self.outgoing.iter())
    }

    fn degree_no_self<'a>(stream: impl Iterator<Item=(&'a String, &'a ConnectionInfo)>) -> Vec<u64> {
        stream
            .map(
                |(k, c)|
                    c.by_type.iter()
                        .filter(|(t, _, _c)| k != t)
                        .map(|(_, _, c)| *c)
                        .sum::<u64>()
            )
            .collect()
    }

    // ##### Degree No Duplicates and No Duplicates No Self ##### 

    pub fn in_degree_no_duplicates(&self) -> Vec<u64> {
        Self::degree_no_duplicates_with_predicate(
            self.incoming.iter(),
            |_, _| true,
            |_| 1
        )
    }

    pub fn out_degree_no_duplicates(&self) -> Vec<u64> {
        Self::degree_no_duplicates_with_predicate(
            self.outgoing.iter(),
            |_, _| true,
            |_| 1
        )
    }

    pub fn in_degree_no_self_no_duplicates(&self) -> Vec<u64> {
        Self::degree_no_duplicates_with_predicate(
            self.incoming.iter(),
            |a, b| a != b,
            |_| 1
        )
    }

    pub fn out_degree_no_self_no_duplicates(&self) -> Vec<u64> {
        Self::degree_no_duplicates_with_predicate(
            self.outgoing.iter(),
            |a, b| a != b,
            |_| 1
        )
    }

    fn degree_no_duplicates_with_predicate<'a, F, T>(
        stream: impl Iterator<Item=(&'a String, &'a ConnectionInfo)>,
        predicate: F,
        transform: T) -> Vec<u64>
    where
        F: Fn(&String, &String) -> bool,
        T: Fn(u64) -> u64
    {
        let mut result = Vec::new();
        for (node, connections) in stream {
            let mut total = 0;
            let mut seen = HashSet::new();
            for (other, _tp, count) in connections.by_type.iter() {
                if seen.contains(other) || !predicate(node, other) {
                    continue;
                }
                seen.insert(other.clone());
                total += transform(*count);
            }
            result.push(total);
        }
        result
    }

    // ##### Degree By Type ##### 
    
    pub fn in_degree_by_type(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.incoming.iter(),
            |_, _| true,
            |c| c
        )
    }
    
    pub fn out_degree_by_type(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.outgoing.iter(),
            |_, _| true,
            |c| c
        )
    }
    
    pub fn in_degree_by_type_no_self(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.incoming.iter(),
            |a, b| a != b,
            |c| c
        )
    }
    
    pub fn out_degree_by_type_no_self(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.outgoing.iter(),
            |a, b| a != b,
            |c| c
        )
    }

    pub fn in_degree_by_type_no_duplicates(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.incoming.iter(),
            |_, _| true,
            |_c| 1
        )
    }

    pub fn out_degree_by_type_no_duplicates(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.outgoing.iter(),
            |_, _| true,
            |_c| 1
        )
    }

    pub fn in_degree_by_type_no_self_no_duplicates(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.incoming.iter(),
            |a, b| a != b,
            |_c| 1
        )
    }

    pub fn out_degree_by_type_no_self_no_duplicates(&self) -> HashMap<DependencyType, Vec<u64>> {
        Self::degree_by_type_with_predicate(
            self.outgoing.iter(),
            |a, b| a != b,
            |_c| 1
        )
    }

    fn degree_by_type_with_predicate<'a, F, T>(
        stream: impl Iterator<Item=(&'a String, &'a ConnectionInfo)>,
        predicate: F,
        transform: T
    ) -> HashMap<DependencyType, Vec<u64>>
    where
        F: Fn(&String, &String) -> bool,
        T: Fn(u64) -> u64
    {
        let mut result = HashMap::new();
        for (node, connections) in stream {
            let mut total_by_type = HashMap::new();
            for (other, tp, count) in connections.by_type.iter() {
                if !predicate(node, other) {
                    continue;
                }
                *total_by_type.entry(*tp).or_insert(0) += transform(*count);
            }
            for (tp, count) in total_by_type {
                result.entry(tp).or_insert_with(Vec::new).push(count);
            }
        }
        result
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Graph Connectivity Analyser 
///////////////////////////////////////////////////////////////////////////////////////////////////

pub struct GraphConnectivityAnalyser {
    #[allow(unused)] node_map: HashMap<String, usize>,
    distances: Vec<i64>
}

impl GraphConnectivityAnalyser {
    pub fn new<K: DependencyGraphKind>(g: &DependencyGraph<K>) -> Self {
        // Floyd-Warshall algorithm
        // Step 1 -- initialize distances
        let node_map = g.vertices().iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect::<HashMap<_, _>>();
        let n = g.vertices().len();
        let mut distances = vec![-1; n * n];
        for i in 0..n {
            distances[i * n + i] = 0;
        }
        for (from, to) in g.edges().keys() {
            let i = *node_map.get(from).unwrap();
            let j = *node_map.get(to).unwrap();
            if i != j {
                distances[i * n + j] = 1;
            }
        }
        // Step 3 -- calculate distances
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    let i_k = distances[i * n + k];
                    let k_j = distances[k * n + j];
                    if i_k != -1 && k_j != -1 {
                        let i_j = distances[i * n + j];
                        if i_j > i_k + k_j {
                            distances[i * n + j] = i_k + k_j;
                        }
                    }
                }
            }
        }
        Self { node_map, distances }
    }
    
    pub fn diameter(&self) -> u64 {
        *self.distances.iter().max().unwrap() as u64
    }
    
    pub fn hops(&self) -> Vec<u64> {
        self.distances.iter()
            .copied()
            .filter(|d| *d != -1)
            .map(|d| d as u64)
            .collect()
    }
}