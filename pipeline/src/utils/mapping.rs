use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct RenameMapping(HashMap<String, String>);

impl RenameMapping {
    pub fn into_inner(self) -> HashMap<String, String> {
        self.0
    }
}

impl FromStr for RenameMapping {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut mapping = HashMap::new();
        for item in s.split(';') {
            if item.is_empty() {
                continue;
            }
            let parts = item.split('=').collect::<Vec<_>>();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid mapping item: {}", item));
            }
            mapping.insert(parts[0].to_string(), parts[1].to_string());
        }
        Ok(Self(mapping))
    }
}
