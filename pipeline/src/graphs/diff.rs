use std::collections::{HashMap, HashSet};
use crate::graphs::{DependencyGraph, DependencyGraphKind, DependencySpec, DependencyType};

pub struct GraphDiff {
    added_vertices: Vec<VertexWithEdges>,
    removed_vertices: Vec<VertexWithEdges>,
    added_edges: Vec<Edge>,
    removed_edges: Vec<Edge>,
}

impl GraphDiff {
    
    #[allow(unused)]
    pub fn format_diff(&self) -> String {
        let mut lines = vec![
            format!("Added vertices: {}", self.added_vertices.len()),
        ];
        lines.extend(
            self.added_vertices.iter().map(|v| v.format())
        );
        lines.push(
            format!("Removed vertices: {}", self.removed_vertices.len())
        );
        lines.extend(
            self.removed_vertices.iter().map(|v| v.format())
        );
        lines.push(
            format!("Added edges: {}", self.added_edges.len())
        );
        lines.extend(
            self.added_edges.iter().map(|e| e.format())
        );
        lines.push(
            format!("Removed edges: {}", self.removed_edges.len())
        );
        lines.extend(
            self.removed_edges.iter().map(|e| e.format())
        );
        lines.join("\n")
    }
}

pub struct VertexWithEdges {
    vertex: Vertex,
    edges: Vec<Edge>,
}

impl VertexWithEdges {
    pub fn format(&self) -> String {
        if self.edges.is_empty() {
            format!(" * {}", self.vertex.format())
        } else {
            format!("* {}:\n     - {}",
                    self.vertex.format(),
                    self.edges.iter()
                        .map(|e| e.format())
                        .collect::<Vec<_>>()
                        .join("\n     - "))
        }
    }
}

pub struct Edge {
    from: Vertex,
    to: Vertex,
    edge_type: DependencyType,
    count: usize
}

impl Edge {
    pub fn format(&self) -> String {
        let kind = match self.edge_type {
            DependencyType::Uses => "uses",
            DependencyType::Extends => "extends",
            DependencyType::Implements => "implements",
            DependencyType::Unspecified => "unspecified"
        };
        format!(" * {} -> {} ({}; x{})",
                self.from.format(),
                self.to.format(),
                kind,
                self.count)
    }
}

pub struct Vertex(String);

impl Vertex {
    pub fn format(&self) -> String {
        self.0.clone()
    }
}


#[allow(unused)]
pub fn diff_graphs<K>(old: &DependencyGraph<K>, new: &DependencyGraph<K>) -> GraphDiff
where
    K: DependencyGraphKind
{
    let removed_vertices = diff_vertices(old, new);
    let added_vertices = diff_vertices(new, old);
    let mut removed_edges= diff_edges(old, new);
    let mut added_edges = diff_edges(new, old);
    let edges_removed_with_vertices = trim_vertex_edges(
        &mut removed_edges, removed_vertices
    );
    let edges_added_with_vertices = trim_vertex_edges(
        &mut added_edges, added_vertices
    );
    GraphDiff {
        added_vertices: convert_vertices(edges_added_with_vertices),
        removed_vertices: convert_vertices(edges_removed_with_vertices),
        added_edges: convert_edges(added_edges),
        removed_edges: convert_edges(removed_edges)
    }
}


fn diff_vertices<K>(lhs: &DependencyGraph<K>, rhs: &DependencyGraph<K>) -> HashSet<String> 
where
    K: DependencyGraphKind
{
    lhs.vertices().iter()
        .filter(|v| !rhs.vertices().contains(*v))
        .cloned()
        .collect()
}

fn diff_edges<K>(lhs: &DependencyGraph<K>,
                 rhs: &DependencyGraph<K>) -> HashMap<(String, String), DependencySpec>
where
    K: DependencyGraphKind
{
    lhs.edges().iter()
        .filter(|(k, _v)| !rhs.edges().contains_key(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn trim_vertex_edges(edges: &mut HashMap<(String, String), DependencySpec>, 
                     vertices: HashSet<String>) -> HashMap<String, HashMap<(String, String), DependencySpec>>
{
    let mut result = HashMap::new();
    for ((from, to), kind) in edges.iter() {
        for key in [from, to] {
            if !vertices.contains(key) {
                result.entry(key.clone()).or_insert_with(HashMap::new)
                    .insert((from.clone(), to.clone()), kind.clone());
            }
        }
    }
    for (_, inner) in result.iter() {
        for (key, _) in inner.iter() {
            edges.remove(key);
        }
    }
    result 
}

fn convert_vertices(
    vertices: HashMap<String, HashMap<(String, String), DependencySpec>>) -> Vec<VertexWithEdges> 
{
    vertices.into_iter()
        .map(|(vertex, edges)| VertexWithEdges {
            vertex: Vertex(vertex),
            edges: convert_edges(edges)
        })
        .collect()
}

fn convert_edges(edges: HashMap<(String, String), DependencySpec>) -> Vec<Edge> {
    edges.into_iter()
        .flat_map(
            |(edge, spec)| 
                spec.edges()
                    .iter()
                    .map(|(kind, count)| Edge {
                        from: Vertex(edge.0.clone()),
                        to: Vertex(edge.1.clone()),
                        edge_type: *kind,
                        count: *count
                    })
                    .collect::<Vec<_>>()
        )
        .collect()
}