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
    pub(super) name_mapping: HashMap<String, String>,
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
            log::info!("Processing minor {} {}", major, minor);
            let mut changes: HashMap<String, Vec<ChangeInfo>> = HashMap::new();
            let mut pairs = HashMap::new();
            let mut co_changes: HashMap<String, Vec<ChangeInfo>> = HashMap::new();
            let mut name_mapping = HashMap::new();
            for commit in data.commits {
                let commit_data = data.commit_change_data.remove(&commit)
                    .unwrap_or_else(|| panic!("Commit {} missing data", commit));
                let change_info = ChangeInfo {
                    seq: commit_data.seq,
                    author_date_ts: commit_data.author_date_ts,
                    committer_date_ts: commit_data.committer_date_ts,
                };
                let all_class_names = commit_data.files.into_iter()
                    .filter(|f| f.package_old.is_some() || f.package_new.is_some())
                    .flat_map(
                        |f| {
                            let package = match (f.package_old.as_ref(), f.package_new.as_ref()) {
                                (Some(_), Some(new)) => new.to_string(),
                                (Some(old), None) => old.to_string(),
                                (None, Some(new)) => new.to_string(),
                                (None, None) => panic!("File {} has no package", f.name)
                            };
                            f.classes_changed.into_iter()
                                .map(move |c| format!("{}.{}", package, c))
                        }
                    )
                    .collect::<Vec<_>>();
                for name in all_class_names.iter() {
                    if !name_mapping.contains_key(name) {
                        name_mapping.insert(name.clone(), format!("{}", name_mapping.len()));
                    }
                }
                let all_classes = all_class_names.into_iter()
                    .map(|n| name_mapping.get(&n).unwrap().clone())
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
                    co_changes,
                    name_mapping
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


