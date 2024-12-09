//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::collections::HashMap;
use crate::datasets::triples::VersionTriple;
use crate::graphs::{ClassGraph, DependencyGraph};
use crate::graphs::hierarchy::Hierarchy;
//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Structs
//////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionTimeSeriesFeatures {
    versions: Vec<DataForVersion>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataForVersion {
    // metadata 
    version: String,
    // author_ts: f64,
    // committer_ts: f64,
    // seq: usize,
    
    // feature data -- link, version level 
    link_changes: HashMap<(String, String), EdgeChangeInfo>,
    
    // feature data -- node, version level
    node_changes: HashMap<String, NodeChangeInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EdgeChangeInfo {
    additions: u64,
    deletions: u64,
    //modified: u64,
    was_new: bool,
    was_removed: bool
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct NodeChangeInfo {
    added_incoming: u64,
    added_outgoing: u64,
    removed_incoming: u64,
    removed_outgoing: u64,
    //modified_incoming: u64,
    //modified_outgoing: u64,
    
    removed_classes: u64,
    added_classes: u64,
    modified_classes: u64,
    
    was_new: bool,
    was_removed: bool
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Implementation
//////////////////////////////////////////////////////////////////////////////////////////////////

impl DataForVersion {
    fn from_successive_versions(v1: DependencyGraph<ClassGraph>, 
                                v2: DependencyGraph<ClassGraph>) -> Self 
    {
        let mut node_changes: HashMap<String, NodeChangeInfo> = HashMap::new();
        let mut link_changes: HashMap<(String, String), EdgeChangeInfo> = HashMap::new();
        // Removed classes 
        for vertex in v1.vertices().difference(v2.vertices()) {
            let (package, _) = vertex.rsplit_once('.')
                .expect("Could not get package");
            let info = node_changes.entry(package.to_string()).or_default();
            info.removed_classes = 1;
            //info.modified_classes = 1;
            info.was_removed = true;
        }
        // Added classes 
        for vertex in v2.vertices().difference(v1.vertices()) {
            let (package, _) = vertex.rsplit_once('.')
                .expect("Could not get package");
            let info = node_changes.entry(package.to_string()).or_default();
            info.added_classes = 1;
            //info.modified_classes = 1;
            info.was_new = true;
        }
        
        // Added incoming/outgoing: through the identification of new edges 
        // Modified class: Class with new dependencies (through identification of new edges)
        // Modified incoming/outgoing: requires git history
        
        
        // Edges should be done, except for "modified" (which requires Git history) 
        let v1_edges = v1.edges();
        for (edge, spec2) in v2.edges() {
            let info = link_changes.entry(edge.clone()).or_default();
            if !v1_edges.contains_key(edge) {
                info.was_new = true;
                info.additions = spec2.edges().values().copied().sum::<usize>() as u64;
            } else {
                let zero = 0usize;
                let mut delta = 0;
                let spec1_edges = v1_edges[edge].edges();
                for (kind, v2_count) in spec2.edges() {
                    let v1_count = *spec1_edges.get(kind).unwrap_or(&zero);
                    if *v2_count > v1_count {
                        delta += *v2_count - v1_count;
                    };
                }
                info.additions = delta as u64;
                
                let cls_info_out = node_changes.entry(edge.0.clone())
                    .or_default();
                if !cls_info_out.was_new {
                    cls_info_out.modified_classes = 1;
                }
                cls_info_out.added_outgoing = delta as u64;
                
                let cls_info_in = node_changes.entry(edge.1.clone())
                    .or_default();
                if !cls_info_in.was_new {
                    cls_info_in.modified_classes = 1;
                }
                cls_info_in.added_incoming = delta as u64;
            }
        }

        let v2_edges = v2.edges();
        for (edge, spec1) in v1.edges() {
            let info = link_changes.entry(edge.clone()).or_default();
            if !v2_edges.contains_key(edge) {
                info.was_removed = true;
                info.deletions = spec1.edges().values().copied().sum::<usize>() as u64;
            } else {
                let zero = 0usize;
                let mut delta = 0;
                let spec2_edges = v2_edges[edge].edges();
                for (kind, v1_count) in spec1.edges() {
                    let v2_count = *spec2_edges.get(kind).unwrap_or(&zero);
                    if *v1_count > v2_count {
                        delta += *v1_count - v2_count;
                    };
                }
                info.deletions = delta as u64;

                let cls_info_out = node_changes.entry(edge.0.clone())
                    .or_default();
                if !cls_info_out.was_removed {
                    cls_info_out.modified_classes = 1;
                }
                cls_info_out.removed_outgoing = delta as u64;
                
                let cls_info_in = node_changes.entry(edge.1.clone())
                    .or_default();
                if !cls_info_in.was_removed {
                    cls_info_in.modified_classes = 1;
                }
                cls_info_in.removed_incoming = delta as u64;
            }
        }
        
        // What information do we need:
        //  1) Added classes per package 
        //  2) Removed classes per package 
        //  3) EditChangeInfo on the class level
        //  4) NodeChangeInfo per class
        //
        // Next:
        //  1) Aggregate EditChangeInfo per package
        //  2) Propagate EditChangeInfo up the chain
        //  3) Aggregate NodeChangeInfo per package 
        //  4) Propagate NodeChangeInfo up the chain
        //      Note that was_new and was_removed are not aggregated,
        //      but computed separately.
        
        
        let package_graph = v2.to_module_graph();
        let structure: Vec<Hierarchy> = package_graph.into();
    }
}
