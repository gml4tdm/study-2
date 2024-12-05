use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use crate::languages::Language;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Commit<T: Clone>
{
    seq: usize,
    author_date_ts: f64,
    committer_date_ts: f64,
    files: Vec<T>
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
    name: String,
    name_old: Option<String>,
    name_new: Option<String>,
    action: String,
    classes_before: Vec<String>,
    classes_after: Vec<String>,
    classes_changed: Vec<String>
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


pub fn convert_raw_history_data(inp: Vec<Commit<FileChangeInfo>>) -> anyhow::Result<Vec<Commit<ClassChangeInfo>>> {
    let converted = inp.into_iter()
        .map(|c| c.get_class_changes())
        .collect::<Vec<_>>();
    
    let mut filtered = Vec::new();
    for mut commit in converted {
        commit = commit.only_code_files()?;
        if !commit.files.is_empty() {
            filtered.push(commit);
        }
    }
    
    Ok(filtered) 
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
