//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::collections::{HashMap, HashSet};
use std::ops::AddAssign;
use std::path::PathBuf;
use crate::graphs::{ClassGraph, DependencyGraph};
use crate::graphs::hierarchy::Hierarchy;
use crate::graphs::loaders::load_graph_from_file;
use crate::utils::versions::ExtractProjectInformation;
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
    links: HashMap<String, (String, String)>,
    link_changes: HashMap<String, EdgeChangeInfo>,
    
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


// Auxiliary struct 
#[derive(Debug, Clone)]
struct SepChangeInfo {
    in_links_added: HashMap<String, HashMap<String, u64>>,
    out_links_added: HashMap<String, HashMap<String, u64>>,
    in_links_removed: HashMap<String, HashMap<String, u64>>,
    out_links_removed: HashMap<String, HashMap<String, u64>>
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Implementation
//////////////////////////////////////////////////////////////////////////////////////////////////

impl EdgeChangeInfo {
    pub fn default_truthy() -> Self {
        Self {
            additions: 0,
            deletions: 0,
            was_new: true,
            was_removed: true
        }
    }
}

impl NodeChangeInfo {
    pub fn default_truthy() -> Self {
        Self {
            added_incoming: 0,
            added_outgoing: 0,
            removed_incoming: 0,
            removed_outgoing: 0,
            removed_classes: 0,
            added_classes: 0,
            modified_classes: 0,
            was_new: true,
            was_removed: true
        }
    }
}

impl VersionTimeSeriesFeatures {
    pub fn new(versions: Vec<PathBuf>) -> anyhow::Result<Self> {
        let mut data = Vec::new();
        let mut versions = versions.into_iter()
            .map(|path| Ok((path.clone(), path.extract_version()?.to_string())))
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        versions.sort_by(|(_, a), (_, b)| crate::utils::versions::cmp_versions(a, b));
        for (v1, v2) in versions.iter().zip(versions.iter().skip(1)) {
            let label = v2.1.clone();
            log::info!("Processing version pair {}, {}", v1.1, v2.1);
            let v1 = load_graph_from_file(v1.0.clone())?;
            let v2 = load_graph_from_file(v2.0.clone())?;
            let x = DataForVersion::from_successive_versions(label, v1, v2);
            data.push(x);
        }
        Ok(Self { versions: data })
    }
}

impl DataForVersion {
    pub fn from_successive_versions(label: String,
                                    v1: DependencyGraph<ClassGraph>, 
                                    v2: DependencyGraph<ClassGraph>) -> Self 
    {
        let mut node_changes: HashMap<String, NodeChangeInfo> = HashMap::new();
        let mut link_changes: HashMap<(String, String), EdgeChangeInfo> = HashMap::new();
        
        let mut in_links_added_per_class: HashMap<String, HashMap<String, u64>> = HashMap::new();
        let mut out_links_added_per_class: HashMap<String, HashMap<String, u64>> = HashMap::new();
        let mut in_links_removed_per_class: HashMap<String, HashMap<String, u64>> = HashMap::new();
        let mut out_links_removed_per_class: HashMap<String, HashMap<String, u64>> = HashMap::new();
        
        // Removed classes 
        for vertex in v1.vertices().difference(v2.vertices()) {
            let info = node_changes.entry(vertex.to_string()).or_default();
            info.removed_classes = 1;
            //info.modified_classes = 1;
            info.was_removed = true;
            log::trace!("Marking removed class: {vertex}");
        }
        // Added classes 
        for vertex in v2.vertices().difference(v1.vertices()) {
            let info = node_changes.entry(vertex.to_string()).or_default();
            info.added_classes = 1;
            //info.modified_classes = 1;
            info.was_new = true;
            log::trace!("Marking added class: {vertex}");
        }
        for vertex in v1.vertices().intersection(v2.vertices()) {
            let _info = node_changes.entry(vertex.to_string()).or_default();
            log::trace!("Marking unchanged class: {vertex}");
        }
        
        // Edges should be done, except for "modified" (which requires Git history) 
        let v1_edges = v1.edges();
        for (edge, spec2) in v2.edges() {
            log::trace!("Processing edge [v2]: {} --> {}", edge.0, edge.1);
            let info = link_changes.entry(edge.clone()).or_default();
            let delta = if !v1_edges.contains_key(edge) {
                log::trace!("Marking edge as new");
                info.was_new = true;
                info.additions = spec2.edges().values().copied().sum::<usize>() as u64;
                in_links_added_per_class.entry(edge.1.clone()).or_default()
                    .entry(edge.0.clone()).or_default()
                    .add_assign(info.additions);
                out_links_added_per_class.entry(edge.0.clone()).or_default()
                    .entry(edge.1.clone()).or_default()
                    .add_assign(info.additions);
                info.additions 
            } else {
                let zero = 0usize;
                let mut delta = 0;
                let spec1_edges = v1_edges[edge].edges();
                for (kind, v2_count) in spec2.edges() {
                    let v1_count = *spec1_edges.get(kind).unwrap_or(&zero);
                    if *v2_count > v1_count {
                        delta += *v2_count - v1_count;
                        in_links_added_per_class.entry(edge.1.clone()).or_default()
                            .entry(edge.0.clone()).or_default()
                            .add_assign(info.additions);
                        out_links_added_per_class.entry(edge.0.clone()).or_default()
                            .entry(edge.1.clone()).or_default()
                            .add_assign(info.additions);
                    };
                }
                if delta > 0 {
                    log::trace!("Marking edge as modified (additions = {delta})");
                    info.additions = delta as u64;
                }
                delta as u64
            };
            if delta > 0 {
                let cls_info_out = node_changes.entry(edge.0.clone())
                    .or_default();
                if !cls_info_out.was_new {
                    log::trace!("Marking outgoing {} class as modified", edge.0);
                    cls_info_out.modified_classes = 1;
                }
            }
        }

        let v2_edges = v2.edges();
        for (edge, spec1) in v1.edges() {
            log::trace!("Processing edge [v1]: {} --> {}", edge.0, edge.1);
            let info = link_changes.entry(edge.clone()).or_default();
            let delta = if !v2_edges.contains_key(edge) {
                log::trace!("Marking edge as removed");
                info.was_removed = true;
                info.deletions = spec1.edges().values().copied().sum::<usize>() as u64;
                in_links_removed_per_class.entry(edge.1.clone()).or_default()
                    .entry(edge.0.clone()).or_default()
                    .add_assign(info.deletions);
                out_links_removed_per_class.entry(edge.0.clone()).or_default()
                    .entry(edge.1.clone()).or_default()
                    .add_assign(info.deletions);
                info.deletions
            } else {
                let zero = 0usize;
                let mut delta = 0;
                let spec2_edges = v2_edges[edge].edges();
                for (kind, v1_count) in spec1.edges() {
                    let v2_count = *spec2_edges.get(kind).unwrap_or(&zero);
                    if *v1_count > v2_count {
                        delta += *v1_count - v2_count;
                        in_links_removed_per_class.entry(edge.1.clone()).or_default()
                            .entry(edge.0.clone()).or_default()
                            .add_assign(info.deletions);
                        out_links_removed_per_class.entry(edge.0.clone()).or_default()
                            .entry(edge.1.clone()).or_default()
                            .add_assign(info.deletions);
                    };
                }
                if delta > 0 {
                    log::trace!("Marking edge as modified (deletions = {delta})");
                    info.deletions = delta as u64;
                }
                delta as u64
            };
            if delta > 0 {
                let cls_info_out = node_changes.entry(edge.0.clone())
                    .or_default();
                if !cls_info_out.was_removed {
                    log::trace!("Marking outgoing {} class as modified", edge.0);
                    cls_info_out.modified_classes = 1;
                }
            }
        }
        
        let package_graph = v2.to_module_graph();
        let structure: Vec<Hierarchy> = package_graph.into();
        
        let mut nodes_per_package: HashMap<String, HashSet<String>> = HashMap::new();
        for vertex in node_changes.keys() {
            let (package, _) = vertex.rsplit_once('.')
                .expect("Could not get package");
            nodes_per_package.entry(package.to_string()).or_default().insert(vertex.to_string());
        }
        
        let mut edges_per_package: HashMap<String, HashSet<(String, String)>> = HashMap::new();
        for edge in link_changes.keys() {
            let (package_1, _) = edge.0.rsplit_once('.')
                .expect("Could not get package");
            let (package_2, _) = edge.1.rsplit_once('.')
                .expect("Could not get package");
            edges_per_package.entry(package_1.to_string()).or_default().insert(edge.clone());
            edges_per_package.entry(package_2.to_string()).or_default().insert(edge.clone());
        }
        
        for h in structure.clone() {
            Self::aggregate_recursively(
                h,
                &mut nodes_per_package,
                &mut edges_per_package,
                &mut node_changes,
                &mut link_changes
            );
        }
        
        let sep = SepChangeInfo {
            in_links_added: in_links_added_per_class,
            out_links_added: out_links_added_per_class,
            in_links_removed: in_links_removed_per_class,
            out_links_removed: out_links_removed_per_class
        };
        for (cls, details) in &sep.in_links_added {
            let info = node_changes.entry(cls.clone()).or_default();
            info.added_incoming += details.values().sum::<u64>();
        }
        for (cls, details) in &sep.out_links_added {
            let info = node_changes.entry(cls.clone()).or_default();
            info.added_outgoing += details.values().sum::<u64>();
        }
        for (cls, details) in &sep.in_links_removed {
            let info = node_changes.entry(cls.clone()).or_default();
            info.removed_incoming += details.values().sum::<u64>();
        }
        for (cls, details) in &sep.out_links_removed {
            let info = node_changes.entry(cls.clone()).or_default();
            info.removed_outgoing += details.values().sum::<u64>();
        }
        for h in structure {
            Self::aggregate_in_out_recursively(
                h,
                &mut node_changes,
                sep.clone()
            );
        }
        
        let mut links = HashMap::new();
        let mut link_changes_mapped = HashMap::new();
        for ((from, to), info) in link_changes {
            let key = format!("{}", links.len());
            links.insert(key.clone(), (from.clone(), to.clone()));
            link_changes_mapped.insert(key, info);
        }
        
        Self { version: label, node_changes, link_changes: link_changes_mapped, links }
    }
    

    
    fn aggregate_in_out_recursively(
        hierarchy: Hierarchy, 
        node_changes: &mut HashMap<String, NodeChangeInfo>,
        changes: SepChangeInfo) 
    {
        let package = hierarchy.name;
        if !hierarchy.children.is_empty() {
            for child in hierarchy.children {
                Self::aggregate_in_out_recursively(
                    child,
                    node_changes,
                    changes.clone()
                );
            }
        } 
        
        let aggregate = &|mapping: &HashMap<String, HashMap<String, u64>>| {
            let mut total = 0u64;
            for (from, details) in mapping {
                if !from.starts_with(&package) {
                    continue;
                }
                for (to, count) in details {
                    if to.starts_with(&package) {
                        continue;
                    }
                    total += *count;
                }
            }
            total
        };
        
        let in_links_added = aggregate(&changes.in_links_added);
        let out_links_added = aggregate(&changes.out_links_added);
        let in_links_removed = aggregate(&changes.in_links_removed);
        let out_links_removed = aggregate(&changes.out_links_removed);
       
        let info = node_changes.entry(package.clone()).or_default();
        info.added_incoming += in_links_added;
        info.added_outgoing += out_links_added;
        info.removed_incoming += in_links_removed;
        info.removed_outgoing += out_links_removed;
    }
    
    fn aggregate_recursively(hierarchy: Hierarchy,
                             nodes_per_package: &mut HashMap<String, HashSet<String>>,
                             edges_per_package: &mut HashMap<String, HashSet<(String, String)>>,
                             node_changes: &mut HashMap<String, NodeChangeInfo>,
                             link_changes: &mut HashMap<(String, String), EdgeChangeInfo>)
    {
        for child in hierarchy.children {
            // Also update children 
            nodes_per_package.entry(hierarchy.name.clone()).or_default().insert(child.name.clone());
            
            edges_per_package.entry(hierarchy.name.clone()).or_default()
                .extend(
                    link_changes.iter()
                        .filter(|((from, to), _info)| {
                            let (from_prefix, _) = from.rsplit_once('.')
                                .expect("Could not get package");
                            let (to_prefix, _) = to.rsplit_once('.')
                                .expect("Could not get package");
                            from_prefix == hierarchy.name || to_prefix == hierarchy.name
                        })
                        .map(|(edge, _info)| edge)
                        .cloned()
                );


            // Aggregate children 
            Self::aggregate_recursively(child,
                                        nodes_per_package,
                                        edges_per_package,
                                        node_changes,
                                        link_changes);
        }

        let package = hierarchy.name;

        // Node aggregation 
        let child_node_info = nodes_per_package
            .entry(package.clone())
            .or_default()
            .iter()
            .map(|v| node_changes.get(v).expect("Node not found"))
            .cloned()
            .collect::<Vec<_>>();
        let node_info = node_changes.entry(package.clone()).or_insert_with(NodeChangeInfo::default_truthy);
        //node_info.added_incoming += child_node_info.iter().map(|i| i.added_incoming).sum::<u64>();
        //node_info.added_outgoing += child_node_info.iter().map(|i| i.added_outgoing).sum::<u64>();
        //node_info.removed_incoming += child_node_info.iter().map(|i| i.removed_incoming).sum::<u64>();
        //node_info.removed_outgoing += child_node_info.iter().map(|i| i.removed_outgoing).sum::<u64>();
        node_info.removed_classes += child_node_info.iter().map(|i| i.removed_classes).sum::<u64>();
        node_info.added_classes += child_node_info.iter().map(|i| i.added_classes).sum::<u64>();
        node_info.modified_classes += child_node_info.iter().map(|i| i.modified_classes).sum::<u64>();
        // Only removed/new if all children are removed/new
        node_info.was_new &= child_node_info.iter().all(|i| i.was_new);
        node_info.was_removed &= child_node_info.iter().all(|i| i.was_removed);

        // Link aggregation 
        let child_link_info = edges_per_package
            .entry(package.clone())
            .or_default()
            .iter()
            .map(|e| (
                e.clone(),
                link_changes.get(e).expect("Link not found").clone()
            ))
            .collect::<Vec<_>>();
        for ((from, to), info) in child_link_info {
            let (from_prefix, _) = from.rsplit_once('.')
                .expect("Could not get package");
            let (to_prefix, _) = to.rsplit_once('.')
                .expect("Could not get package");
            
            let key = if from_prefix == package && to_prefix == package {
                (from_prefix.to_string(), to_prefix.to_string())
            } else if from_prefix == package {
                (from_prefix.to_string(), to.clone())
            } else if to_prefix == package {
                (from.clone(), to_prefix.to_string())
            } else {
                panic!("Cannot determine aggregation relation");
            };
            
            let cur_info = link_changes.entry(key).or_insert_with(EdgeChangeInfo::default_truthy);
            cur_info.additions += info.additions;
            cur_info.deletions += info.deletions;
            cur_info.was_new &= info.was_new;           // only new if all child links are new 
            cur_info.was_removed &= info.was_removed;   // Only removed if all child links are removed
        }
    }
}
