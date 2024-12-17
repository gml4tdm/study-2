use std::collections::HashMap;
use crate::datasets::co_change::CoChangeDataset;

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairCoChangeInfo {
    lifetime_change_likelihood: f64,
    lifetime_damped_change_likelihood: f64,
    version_change_likelihood: f64,
    version_damped_change_likelihood: f64,
    
    commits_since_last_change: u64,
    time_since_last_change: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnitCoChangeInfo {
    commits_since_last_change: u64,
    time_since_last_change: f64,
    
    lifetime_co_change_prospect: f64,
    lifetime_co_change_prospect_damped: f64,
    version_co_change_prospect: f64,
    version_co_change_prospect_damped: f64,
}

pub fn generate_co_change_features(CoChangeDataset(data): CoChangeDataset) -> CoChangeFeatureDataset {
    let mut result = HashMap::new();
    for (major, minors) in data.into_iter() {
        let mut mapping = HashMap::new();
        
        // For the (co-) change sets and other features, we make use of the 
        // fact that the seq field is unique per CoChangeVersion.
        // For every CoChangeVersion, seq resets.
        
        let mut modifications = HashMap::new();
        let mut co_modifications = HashMap::new();
        let mut global_sequence = 0;
        
        for (minor, version) in minors.into_iter() {
            mapping.insert(minor.clone(), CoChangeFeatures {
                old: version.old_version,
                new: version.new_version,
                pairs: version.changes.pairs,
                paired_features: HashMap::new(),
                unit_features: HashMap::new(),
            });
        }
    }
    CoChangeFeatureDataset(result)
}
