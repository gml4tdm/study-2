use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use itertools::Itertools;
use crate::datasets::co_change::CoChangeDataset;
use crate::graphs::{ClassGraph, DependencyGraph};
use crate::graphs::hierarchy::Hierarchy;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoChangeFeatureDataset(HashMap<String, HashMap<String, CoChangeFeatures>>);


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoChangeFeatures {
    old: String,
    new: String,
    pairs: HashMap<String, (String, String)>,
    paired_features: HashMap<String, PairCoChangeInfo>,
    unit_features: HashMap<String, UnitCoChangeInfo>,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairCoChangeInfo {
    lifetime_change_likelihood: f64,
    //lifetime_damped_change_likelihood: f64,
    version_change_likelihood: f64,
    //version_damped_change_likelihood: f64,
    
    //commits_since_last_change: u64,
    //time_since_last_change: f64,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnitCoChangeInfo {
    //commits_since_last_change: u64,
    time_since_last_change: f64,
    
    lifetime_co_change_prospect: f64,
    //lifetime_co_change_prospect_damped: f64,
    version_co_change_prospect: f64,
    //version_co_change_prospect_damped: f64,
}

pub fn generate_co_change_features_2(CoChangeDataset(data): CoChangeDataset, 
                                   graphs: HashMap<(String, String), DependencyGraph<ClassGraph>>) -> CoChangeFeatureDataset
{
    let mut result = HashMap::new();
    for (major, minors) in data.into_iter() {
        
        let mut mapping = HashMap::new();
        
        // For correct lifetime ordering, we need to sort the minors by version.
        // Note that these are "simply" integers encoded as strings.
        let mut ordered_minors = minors.into_iter().collect::<Vec<_>>();
        ordered_minors.sort_by_key(
            |(v, _)| u64::from_str(v).unwrap_or_else(|_| panic!("Could not parse version {}", v))
        );
        
        // For the (co-) change sets and other features, we make use of the 
        // fact that the seq field is unique per CoChangeVersion.
        // For every CoChangeVersion, seq resets.
        
        let mut modifications: HashMap<String, HashSet<usize>> = HashMap::new();
        let mut co_modifications: HashMap<(String, String), HashSet<usize>> = HashMap::new();
        let mut lifetime_classes: HashSet<String> = HashSet::new();
        let mut modification_times: HashMap<String, f64> = HashMap::new();
        let mut co_modification_times: HashMap<(String, String), f64> = HashMap::new();
        let mut global_sequence = 0;
        
        for (minor, version) in ordered_minors.into_iter() {
            log::info!("Generating Co-Change features for {}.{}", major, minor);
            
            let mut version_modifications: HashMap<String, HashSet<usize>> = HashMap::new();
            let mut version_co_modifications: HashMap<(String, String), HashSet<usize>> = HashMap::new();
            let mut version_classes: HashSet<String> = HashSet::new();
            let mut offset = 0;
            
            let mut release_time = 0.0f64;
            
            let name_mapping = version.changes.name_mapping.into_iter()
                .map(|(k, v)| (v, k))
                .collect::<HashMap<_, _>>();

            for (raw_component, changes) in version.changes.changes {
                lifetime_classes.insert(raw_component.clone());
                version_classes.insert(raw_component.clone());
                let component = name_mapping
                    .get(&raw_component)
                    .unwrap_or_else(|| panic!("Component {raw_component} not found in name mapping"));
                for change in changes {
                    modification_times.entry(component.clone())
                        .and_modify(|e| { *e = e.max(change.committer_date_ts) })
                        .or_insert(0.0);
                    modifications.entry(component.clone())
                        .or_default()
                        .insert(global_sequence + change.seq);
                    version_modifications.entry(component.clone())
                        .or_default()
                        .insert(global_sequence + change.seq);
                    offset = offset.max(change.seq);
                    release_time = release_time.max(change.committer_date_ts);
                }
            }
            
            for (key, changes) in version.changes.co_changes {
                // let key = version.changes.pairs.get(&key)
                //     .unwrap_or_else(|| panic!("Key {} missing from pairs", key));
                let raw_key = version.changes.pairs.get(&key)
                    .expect("Key not found in pairs");
                lifetime_classes.insert(raw_key.0.clone());
                version_classes.insert(raw_key.1.clone());
                let key = (
                        name_mapping
                            .get(&raw_key.0)
                            .expect("Component not found in name mapping")
                            .clone(),
                        name_mapping
                            .get(&raw_key.1)
                            .expect("Component not found in name mapping")
                            .clone()
                    );
                for change in changes {
                    co_modification_times.entry(key.clone())
                        .and_modify(|e| { *e = e.max(change.committer_date_ts) })
                        .or_insert(0.0);
                    co_modifications.entry(key.clone())
                        .or_default()
                        .insert(global_sequence + change.seq);
                    version_co_modifications.entry(key.clone())
                        .or_default()
                        .insert(global_sequence + change.seq);
                    offset = offset.max(change.seq);
                    release_time = release_time.max(change.committer_date_ts);
                }
            }
            
            global_sequence += offset + 1;
            
            let mut paired_features = HashMap::new();
            let mut changes_by_cls: HashMap<String, Vec<PairCoChangeInfo>> = HashMap::new();
            let mut pairs = HashMap::new();
            let mut pair_count = 0;
            let key = (major.clone(), minor.clone());
            let graph = graphs.get(&key)
                .unwrap_or_else(|| panic!("Graph for {key:?} not found"));
            //let empty = HashSet::new();

            let mod_graph = graph.to_module_graph();
            let structure: Vec<Hierarchy> = mod_graph.into();
            let packages = structure.into_iter()
                .flat_map(|h| linearize_hierarchy(h))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            
            for (a, b) in packages.iter().cartesian_product(packages.iter()) {
                let info = if a == b {
                    PairCoChangeInfo {
                        lifetime_change_likelihood: 0.0,
                        version_change_likelihood: 0.0,
                    }
                } else {
                    // let v_denom_a = version_modifications.get(a).unwrap_or(&empty);
                    // let v_denom_b = version_modifications.get(b).unwrap_or(&empty);
                    // let v_num = version_co_modifications.get(&(a.clone(), b.clone()))
                    //     .unwrap_or(&empty);
                    // let v_denom = v_denom_a.union(v_denom_b).collect::<HashSet<_>>().len() as f64;
                    // let version_change_likelihood = if v_denom > 0.0 { v_num.len() as f64 / v_denom } else { 0.0 };
                    
                    let version_change_likelihood = change_likelihood(
                        a, b, &version_modifications, &version_co_modifications
                    );
                    
                    // let g_denom_a = modifications.get(a).unwrap_or(&empty);
                    // let g_denom_b = modifications.get(b).unwrap_or(&empty);
                    // let g_num = co_modifications.get(&(a.clone(), b.clone()))
                    //     .unwrap_or(&empty);
                    // let g_denom = g_denom_a.union(g_denom_b).collect::<HashSet<_>>().len() as f64;
                    // let lifetime_change_likelihood = if g_denom > 0.0 { g_num.len() as f64 / g_denom } else { 0.0 };
                    
                    let lifetime_change_likelihood = change_likelihood(
                        a, b, &modifications, &co_modifications
                    );
                    
                    let i = PairCoChangeInfo { lifetime_change_likelihood, version_change_likelihood };

                    changes_by_cls.entry(a.clone())
                        .or_default()
                        .push(i);
                    changes_by_cls.entry(b.clone())
                        .or_default()
                        .push(i);
                    
                    i
                };
                let pair_key = pairs.entry((a.clone(), b.clone()))
                    .or_insert_with(|| {
                        pair_count += 1;
                        format!("{}", pair_count)
                    });
                paired_features.insert(pair_key.clone(), info);
            }

            let mut unit_features = HashMap::new();

            let n = lifetime_classes.len() as f64;
            let n_v = version_classes.len() as f64;
            
            for cls in packages.iter() {
                let info = UnitCoChangeInfo {
                    time_since_last_change: release_time - modification_times.iter()
                        .filter(|(k, _)| is_child(cls, k))
                        .map(|(_, v)| *v)
                        .fold(f64::NEG_INFINITY, f64::max),
                    lifetime_co_change_prospect: changes_by_cls.get(cls)
                        .map(|changes| {
                            changes.iter()
                                .map(|c| c.lifetime_change_likelihood)
                                .sum::<f64>()
                        })
                        .map(|x| x / (n - 1.0))
                        .unwrap_or(0.0),
                    version_co_change_prospect: changes_by_cls.get(cls)
                        .map(|changes| {
                            changes.iter()
                                .map(|c| c.version_change_likelihood)
                                .sum::<f64>()
                        })
                        .map(|x| x / (n_v - 1.0))
                        .unwrap_or(0.0),
                };
                unit_features.insert(cls.clone(), info);
            }
            
            mapping.insert(minor.clone(), CoChangeFeatures {
                old: version.old_version,
                new: version.new_version,
                pairs: pairs.into_iter().map(|(k, v)| (v, k)).collect(),
                paired_features,
                unit_features
            });
        }
        result.insert(major, mapping);
    }
    CoChangeFeatureDataset(result)
}


fn is_child(parent_name: &String, child_name: &String) -> bool {
    let result = parent_name == child_name || child_name.starts_with(format!("{parent_name}.").as_str());
    result 
}


fn linearize_hierarchy(hierarchy: Hierarchy) -> Vec<String> {
    let mut result = Vec::new();
    result.push(hierarchy.name);
    for child in hierarchy.children {
        result.extend(linearize_hierarchy(child.clone()));
    }
    result
}


fn change_likelihood(package_a: &String,
                     package_b: &String,
                     changes: &HashMap<String, HashSet<usize>>, 
                     co_changes: &HashMap<(String, String), HashSet<usize>>) -> f64
{
    // Name mapping: maps number to name 
    let a_changes = changes.into_iter()
        .filter(|(k, _)| is_child(package_a, k))
        .flat_map(|(_, v)| v)
        .copied()
        .collect::<HashSet<_>>();
    let b_changes = changes.into_iter()
        .filter(|(k, _)| is_child(package_b, k))
        .flat_map(|(_, v)| v)
        .copied()
        .collect::<HashSet<_>>();
    
    let n = a_changes.intersection(&b_changes).count() as f64;
    
    if n == 0.0 {
        return 0.0;
    }
    
    let co_changes = co_changes.into_iter()
        .filter(|((k1, k2), _)| {
            is_child(package_a, k1) && is_child(package_b, k2)
        })
        .flat_map(|(_,  v)| v)
        .copied()
        .collect::<HashSet<_>>();
    
    let n_co = co_changes.len() as f64;
    n_co / n
}