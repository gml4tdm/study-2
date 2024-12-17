use std::collections::HashMap;
use itertools::Itertools;
use crate::processing::history::{ClassChangeInfo, History};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoChangeDataset(pub(super) HashMap<String, HashMap<String, CoChangeVersion>>);


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoChangeVersion {
    pub(super) old_version: String,
    pub(super) new_version: String,
    pub(super) changes: CoChangeData
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoChangeData {
    pub(super) changes: HashMap<String, Vec<ChangeInfo>>,
    pub(super) pairs: HashMap<String, (String, String)>,
    pub(super) co_changes: HashMap<String, Vec<ChangeInfo>>
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeInfo {
    pub seq: usize,
    pub author_date_ts: f64,
    pub committer_date_ts: f64,
}


pub fn extract_co_change_history(history: History<ClassChangeInfo>) -> CoChangeDataset {
    let mut result_mapping = HashMap::new();
    
    for (major, minors) in history.0.into_iter() {
        let mut mapping = HashMap::new();
        for (minor, mut data) in minors.into_iter() {
            let mut changes: HashMap<String, Vec<ChangeInfo>> = HashMap::new();
            let mut pairs = HashMap::new();
            let mut co_changes: HashMap<String, Vec<ChangeInfo>> = HashMap::new();
            for commit in data.commits {
                let commit_data = data.commit_change_data.remove(&commit)
                    .unwrap_or_else(|| panic!("Commit {} missing data", commit));
                let change_info = ChangeInfo {
                    seq: commit_data.seq,
                    author_date_ts: commit_data.author_date_ts,
                    committer_date_ts: commit_data.committer_date_ts,
                };
                let all_classes = commit_data.files.into_iter()
                    .flat_map(|f| f.classes_changed)
                    .collect::<Vec<_>>();
                for classes in all_classes.iter().combinations(2) {
                    assert_eq!(classes.len(), 2);
                    for x in PrefixIterator::split(classes[0].clone(), ".".to_string()) {
                        for y in PrefixIterator::split(classes[1].clone(), ".".to_string()) {
                            if x == y { continue; }
                            let (x, y) = if x < y { (x.clone(), y) } else { (y, x.clone()) };
                            let id = format!("{}", pairs.len());
                            pairs.insert(id.clone(), (x.clone(), y));
                            co_changes.entry(id).or_default().push(change_info);
                        }
                    }
                }
                for cls in all_classes {
                    for x in PrefixIterator::split(cls.clone(), ".".to_string()) {
                        changes.entry(x).or_default().push(change_info);
                    }
                }

            }
            mapping.insert(minor.clone(), CoChangeVersion {
                old_version: data.version_old,
                new_version: data.version_new,
                changes: CoChangeData {
                    changes,
                    pairs,
                    co_changes
                }
            });
        }
        result_mapping.insert(major, mapping);
    }
    
    CoChangeDataset(result_mapping)
}


struct PrefixIterator {
    parts: Vec<String>,
    separator: String,
    current: usize
}

impl PrefixIterator {
    pub fn new(parts: Vec<String>, separator: String) -> Self {
        Self { parts, separator, current: 0 }
    }
    
    pub fn split(s: String, by: String) -> Self {
        let parts = s.split(&by).map(|s| s.to_string()).collect::<Vec<_>>();
        Self::new(parts, by)
    }
}

impl Iterator for PrefixIterator {
    type Item = String;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.parts.len() {
            None
        } else {
            let prefix = self.parts[..self.current+1]
                .join(&self.separator.to_string());
            self.current += 1;
            Some(prefix)
        }
    }
}


