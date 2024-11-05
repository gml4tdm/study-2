use std::collections::HashMap;
use crate::graphs::{DependencyGraph, DependencyGraphKind, DependencySpec};
use crate::statistics::shared::{GraphConnectivityAnalyser, GraphDegreeAnalyser, Statistics};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectEvolutionStatistics {
    // Project information
    project: String,
    versions: Vec<String>,
    
    // Graph level statistics
    graphs_per_version: Vec<GraphStatistics>,

    // Time series statistics
    vertices_per_version: Vec<VertexStatistics>,
    edges_per_version: Vec<EdgeStatistics>,
    vertex_edits_per_version: Vec<VertexEditStatistics>,
    edge_edits_per_version: Vec<EdgeEditStatistics>,
}

#[derive(Debug, Copy, Clone, serde::Serialize)]
pub struct GraphStatistics {
    diameter: u64,
    hops: Statistics
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VertexStatistics {
    // Summary statistics
    total: u64,

    // Degree statistics
    in_degree: Statistics,
    out_degree: Statistics,
    in_degree_no_duplicates: Statistics,
    out_degree_no_duplicates: Statistics,
    in_degree_no_self: Statistics,
    out_degree_no_self: Statistics,
    in_degree_no_self_no_duplicates: Statistics,
    out_degree_no_self_no_duplicates: Statistics,
    
    in_degree_by_type: HashMap<String, Statistics>,
    in_degree_by_type_no_self: HashMap<String, Statistics>,
    in_degree_by_type_no_duplicates: HashMap<String, Statistics>,
    in_degree_by_type_no_self_no_duplicates: HashMap<String, Statistics>,
    out_degree_by_type: HashMap<String, Statistics>,
    out_degree_by_type_no_self: HashMap<String, Statistics>,
    out_degree_by_type_no_duplicates: HashMap<String, Statistics>,
    out_degree_by_type_no_self_no_duplicates: HashMap<String, Statistics>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EdgeStatistics {
    total: u64,
    total_no_duplicates: u64,
    total_no_self: u64,
    total_no_self_no_duplicates: u64,
    
    total_by_type: HashMap<String, u64>,
    total_by_type_no_duplicates: HashMap<String, u64>,
    total_by_type_no_self: HashMap<String, u64>,
    total_by_type_no_self_no_duplicates: HashMap<String, u64>,
}

#[derive(Debug, Copy, Clone, serde::Serialize)]
pub struct VertexEditStatistics {
    added: u64,
    deleted: u64
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EdgeEditStatistics {
    added: u64,
    added_no_duplicates: u64,
    added_no_self: u64,
    added_no_self_no_duplicates: u64,
    added_by_type: HashMap<String, u64>,
    added_by_type_no_duplicates: HashMap<String, u64>,
    added_by_type_no_self: HashMap<String, u64>,
    added_by_type_no_self_no_duplicates: HashMap<String, u64>,
    
    deleted: u64,
    deleted_no_duplicates: u64,
    deleted_no_self: u64,
    deleted_no_self_no_duplicates: u64,
    deleted_by_type: HashMap<String, u64>,
    deleted_by_type_no_duplicates: HashMap<String, u64>,
    deleted_by_type_no_self: HashMap<String, u64>,
    deleted_by_type_no_self_no_duplicates: HashMap<String, u64>,    
}


pub fn get_project_evolution_statistics<K>(project: &str, 
                                           versions: &[String],
                                           graphs: &[DependencyGraph<K>]) -> ProjectEvolutionStatistics
where
    K: DependencyGraphKind
{
    let graphs_per_version = graphs.iter()
        .map(|graph| get_graph_statistics(graph))
        .collect::<Vec<_>>();
    let vertices_per_version = graphs.iter()
        .map(|graph| get_vertex_statistics(graph))
        .collect::<Vec<_>>();
    let edges_per_version = graphs.iter()
        .map(|graph| get_edge_statistics(graph))
        .collect::<Vec<_>>();
    let vertex_edits_per_version = graphs.windows(2)
        .map(|graphs| get_vertex_edit_statistics(&graphs[0], &graphs[1]))
        .collect::<Vec<_>>();
    let edge_edits_per_version = graphs.windows(2)
        .map(|graphs| get_edge_edit_statistics(&graphs[0], &graphs[1]))
        .collect::<Vec<_>>();
    
    ProjectEvolutionStatistics {
        project: project.to_string(),
        versions: versions.to_vec(),
        graphs_per_version,
        vertices_per_version,
        edges_per_version,
        vertex_edits_per_version,
        edge_edits_per_version
    }
}

fn get_graph_statistics<K>(graph: &DependencyGraph<K>) -> GraphStatistics
where
    K: DependencyGraphKind
{
    let analyser = GraphConnectivityAnalyser::new(graph);
    GraphStatistics {
        diameter: analyser.diameter(),
        hops: Statistics::from(analyser.hops().into_iter().map(|x| x as f64))
    }
}

fn get_vertex_statistics<K>(graph: &DependencyGraph<K>) -> VertexStatistics
where
    K: DependencyGraphKind
{
    let total = graph.vertices().len();
    let analyser = GraphDegreeAnalyser::new(graph);
    
    VertexStatistics {
        total: total as u64,
        in_degree: Statistics::from(
            analyser.in_degree().into_iter().map(|x| x as f64)
        ),
        out_degree: Statistics::from(
            analyser.out_degree().into_iter().map(|x| x as f64)
        ),
        in_degree_no_duplicates: Statistics::from(
            analyser.in_degree_no_duplicates().into_iter().map(|x| x as f64)
        ),
        out_degree_no_duplicates: Statistics::from(
            analyser.out_degree_no_duplicates().into_iter().map(|x| x as f64)
        ),
        in_degree_no_self: Statistics::from(
            analyser.in_degree_no_self().into_iter().map(|x| x as f64)
        ),
        out_degree_no_self: Statistics::from(
            analyser.out_degree_no_self().into_iter().map(|x| x as f64)
        ),
        in_degree_no_self_no_duplicates: Statistics::from(
            analyser.in_degree_no_self_no_duplicates().into_iter().map(|x| x as f64)
        ),
        out_degree_no_self_no_duplicates: Statistics::from(
            analyser.out_degree_no_self_no_duplicates().into_iter().map(|x| x as f64)
        ),
       in_degree_by_type: analyser.in_degree_by_type().into_iter()
           .map(|(tp, counts)| (
               tp.to_string(), 
               Statistics::from(counts.into_iter().map(|x| x as f64)))
           )
           .collect(),
        in_degree_by_type_no_self: analyser.in_degree_by_type_no_self().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(), 
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect(),
        in_degree_by_type_no_duplicates: analyser.in_degree_by_type_no_duplicates().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(), 
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect(),
        in_degree_by_type_no_self_no_duplicates: analyser.in_degree_by_type_no_self_no_duplicates().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(), 
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect(),
        out_degree_by_type: analyser.out_degree_by_type().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(),
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect(),
        out_degree_by_type_no_self: analyser.out_degree_by_type_no_self().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(),
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect(),
        out_degree_by_type_no_duplicates: analyser.out_degree_by_type_no_duplicates().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(),
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect(),
        out_degree_by_type_no_self_no_duplicates: analyser.out_degree_by_type_no_self_no_duplicates().into_iter()
            .map(|(tp, counts)| (
                tp.to_string(),
                Statistics::from(counts.into_iter().map(|x| x as f64)))
            )
            .collect()
    }
}

fn get_edge_statistics<K>(graph: &DependencyGraph<K>) -> EdgeStatistics
where
    K: DependencyGraphKind
{
    let analyser = GraphDegreeAnalyser::new(graph);
    
    EdgeStatistics {
        total: analyser.in_degree()
            .into_iter().sum(),
        total_no_duplicates: analyser.in_degree_no_duplicates()
            .into_iter().sum(),
        total_no_self: analyser.in_degree_no_self()
            .into_iter().sum(),
        total_no_self_no_duplicates: analyser.in_degree_no_self_no_duplicates()
            .into_iter().sum(),
        total_by_type: analyser.in_degree_by_type().into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().sum()))
            .collect(),
        total_by_type_no_duplicates: analyser.in_degree_by_type_no_duplicates().into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().sum()))
            .collect(),
        total_by_type_no_self: analyser.in_degree_by_type_no_self().into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().sum()))
            .collect(),
        total_by_type_no_self_no_duplicates: analyser.in_degree_by_type_no_self_no_duplicates().into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().sum()))
            .collect(),
    }
}

fn get_vertex_edit_statistics<K>(old: &DependencyGraph<K>, new: &DependencyGraph<K>) -> VertexEditStatistics
where
    K: DependencyGraphKind
{
    let deleted = old.vertices().difference(new.vertices());
    let added = new.vertices().difference(old.vertices());
    
    VertexEditStatistics {
        deleted: deleted.count() as u64,
        added: added.count() as u64,
    }
}

fn get_edge_edit_statistics<K>(old: &DependencyGraph<K>, new: &DependencyGraph<K>) -> EdgeEditStatistics
where
    K: DependencyGraphKind
{
    let (added,
        added_no_duplicates,
        added_no_self,
        added_no_self_no_duplicates,
        added_by_type,
        added_by_type_no_duplicates,
        added_by_type_no_self,
        added_by_type_no_self_no_duplicates) = edge_edit_helper(old, new);
    let (deleted,
        deleted_no_duplicates,
        deleted_no_self,
        deleted_no_self_no_duplicates,
        deleted_by_type,
        deleted_by_type_no_duplicates,
        deleted_by_type_no_self,
        deleted_by_type_no_self_no_duplicates) = edge_edit_helper(new, old);
    EdgeEditStatistics {
        added,
        added_no_duplicates,
        added_no_self,
        added_no_self_no_duplicates,
        added_by_type,
        added_by_type_no_duplicates,
        added_by_type_no_self,
        added_by_type_no_self_no_duplicates,
        deleted,
        deleted_no_duplicates,
        deleted_no_self,
        deleted_no_self_no_duplicates,
        deleted_by_type,
        deleted_by_type_no_duplicates,
        deleted_by_type_no_self,
        deleted_by_type_no_self_no_duplicates,
    }
}


fn edge_edit_helper<K>(lhs: &DependencyGraph<K>, rhs: &DependencyGraph<K>) -> (
    u64, u64, u64, u64, HashMap<String, u64>, HashMap<String, u64>, HashMap<String, u64>, HashMap<String, u64>)
where
    K: DependencyGraphKind
{
    let mut added: u64 = 0;
    let mut added_no_duplicates: u64 = 0;
    let mut added_no_self: u64 = 0;
    let mut added_no_self_no_duplicates: u64 = 0;
    let mut added_by_type: HashMap<String, u64> = HashMap::new();
    let mut added_by_type_no_duplicates: HashMap<String, u64> = HashMap::new();
    let mut added_by_type_no_self: HashMap<String, u64> = HashMap::new();
    let mut added_by_type_no_self_no_duplicates: HashMap<String, u64> = HashMap::new();
    
    let empty_spec = DependencySpec::default();
    for (key, spec) in lhs.edges() {
        let other = rhs.edges().get(key).unwrap_or(&empty_spec);
        let mut any_added = false;
        for (kind, &count) in spec.edges() {
            let other_count = *other.edges().get(kind).unwrap_or(&0);
            if count <= other_count {
                continue;
            }
            any_added = true;
            added += (count - other_count) as u64;
            if key.0 != key.1 {
                added_no_self += (count - other_count) as u64;
            }
            *added_by_type.entry(kind.to_string()).or_insert(0) += (count - other_count) as u64;
            *added_by_type_no_duplicates.entry(kind.to_string()).or_insert(0) += 1;
            if key.0 != key.1 {
                *added_by_type_no_self.entry(kind.to_string()).or_insert(0) += (count - other_count) as u64;
                *added_by_type_no_self_no_duplicates.entry(kind.to_string()).or_insert(0) += 1;
            }
        }
        if any_added {
            added_no_duplicates += 1;
            if key.0 != key.1 {
                added_no_self_no_duplicates += 1;
            }
        }
    }
    (
        added,
        added_no_duplicates,
        added_no_self,
        added_no_self_no_duplicates,
        added_by_type,
        added_by_type_no_duplicates,
        added_by_type_no_self,
        added_by_type_no_self_no_duplicates,
    )
}