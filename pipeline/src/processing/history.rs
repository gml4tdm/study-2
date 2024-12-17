use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use crate::languages::Language;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct History<T: Clone>(pub HashMap<String, HashMap<String, VersionHistory<T>>>);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionHistory<T: Clone> {
    pub commits: Vec<String>,
    pub commit_change_data: HashMap<String, Commit<T>>,
    #[serde(rename = "old-version")] pub version_old: String,
    #[serde(rename = "new-version")] pub version_new: String
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Commit<T: Clone>
{
    pub seq: usize,
    pub author_date_ts: f64,
    pub committer_date_ts: f64,
    pub files: Vec<T>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileChangeInfo {
    name: String,
    name_old: Option<String>,
    name_new: Option<String>,
    action: String,
    methods_before: Vec<String>,
    methods_after: Vec<String>,
    methods_changed: Vec<String>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassChangeInfo {
    pub name: String,
    pub name_old: Option<String>,
    pub name_new: Option<String>,
    pub action: String,
    pub classes_before: Vec<String>,
    pub classes_after: Vec<String>,
    pub classes_changed: Vec<String>
}

impl From<FileChangeInfo> for ClassChangeInfo {
    fn from(info: FileChangeInfo) -> Self {
        Self {
            name: info.name,
            name_old: info.name_old,
            name_new: info.name_new,
            action: info.action,
            classes_before: classes_from_methods(info.methods_before),
            classes_after: classes_from_methods(info.methods_after),
            classes_changed: classes_from_methods(info.methods_changed)
        }
    }
}


fn classes_from_methods(methods: Vec<String>) -> Vec<String> {
    methods.into_iter()
        .filter_map(|m| {
            let (cls, _) = m.rsplit_once("::")?;
            Some(cls.to_string())
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

impl History<FileChangeInfo> {
    pub fn get_class_changes(&self) -> anyhow::Result<History<ClassChangeInfo>> {  
        let changes = self.0.iter()
            .map(|(k, v)| {
                let v = v.iter()
                    .map(|(k, v)| {
                        let v = v.get_class_changes()?;
                        Ok((k.clone(), v))
                    })
                    .collect::<Result<HashMap<_, _>, anyhow::Error>>()?;
                Ok((k.clone(), v))
            })
            .collect::<Result<HashMap<_, _>, anyhow::Error>>()?;
        Ok(History(changes))
    }
}


impl VersionHistory<FileChangeInfo> {
    pub fn get_class_changes(&self) -> anyhow::Result<VersionHistory<ClassChangeInfo>> {
        let (filtered, removed) = convert_raw_history_data(self.commit_change_data.clone())?;
        let result = VersionHistory {
            commits: self.commits.clone().into_iter().filter(|s| !removed.contains(s)).collect(),
            commit_change_data: filtered,
            version_old: self.version_old.clone(),
            version_new: self.version_new.clone()
        };
        Ok(result)
    }
}


impl Commit<FileChangeInfo> {
    pub fn get_class_changes(&self) -> Commit<ClassChangeInfo> {
        Commit {
            seq: self.seq,
            author_date_ts: self.author_date_ts,
            committer_date_ts: self.committer_date_ts,
            files: self.files.iter().map(|f| f.clone().into()).collect()
        }
    }
}


fn convert_raw_history_data(inp: HashMap<String, Commit<FileChangeInfo>>) -> anyhow::Result<(HashMap<String, Commit<ClassChangeInfo>>, Vec<String>)> {
    let converted = inp.into_iter()
        .map(|(k, v)| (k, v.get_class_changes()))
        .collect::<HashMap<_, _>>();
    
    let mut removed = Vec::new();
    let mut filtered = HashMap::new();
    for (key, mut commit) in converted {
        commit = commit.only_code_files()?;
        if !commit.files.is_empty() {
            filtered.insert(key, commit);
        } else {
            removed.push(key);
        }
    }
    
    Ok((filtered, removed)) 
}


impl Commit<ClassChangeInfo> {
    fn only_code_files(self) -> anyhow::Result<Self> {
        let code_files = self.files.into_iter()
            .map(|f| {
                let lan = Language::sniff_from_path(PathBuf::from_str(&f.name)?);
                let b = match lan {
                    Some(l) => l.is_code(),
                    None => false
                };
                Ok((b, f))
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?
            .into_iter()
            .filter(|(b, _f) | *b)
            .map(|(_, f)| f)
            .collect();
        Ok(Self {
            seq: self.seq,
            author_date_ts: self.author_date_ts,
            committer_date_ts: self.committer_date_ts,
            files: code_files
        })
    }
}
