use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Language {
    Java
}

impl FromStr for Language {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "java" => Ok(Language::Java),
            _ => Err(anyhow::anyhow!("Invalid language: {}", s))
        }
    }
}


impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Java => write!(f, "java")
        }
    }
}


impl Language {
    pub fn is_source_file(&self, path: impl AsRef<Path>) -> bool {
        match self {
            Language::Java => path.as_ref().extension().unwrap_or_default() == "java"
        }
    }
    
    pub fn is_code(&self) -> bool {
        match self {
            Self::Java => true 
        }    
    }

    pub fn sniff_from_path(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref().extension()?
            .to_str()?
            .parse::<Self>()
            .ok()
    }
}
