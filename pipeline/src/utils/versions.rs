use std::cmp::Ordering;
use std::path::Path;
use std::sync::OnceLock;
use itertools::{EitherOrBoth, Itertools};
use crate::utils::paths::ExtractFileName;

static FILENAME_PATTERN: OnceLock<regex::Regex> = OnceLock::new();

fn get_filename_pattern() -> &'static regex::Regex {
    FILENAME_PATTERN.get_or_init(|| regex::Regex::new(
        r"^(?<project>[a-zA-Z0-9_\-]+)-(?<version>\d+(\.[a-zA-Z0-9]+)*)$"
    ).unwrap())
}

pub trait ExtractProjectInformation: ExtractFileName {
    fn extract_version(&self) -> anyhow::Result<&str>;
    fn extract_project(&self) -> anyhow::Result<&str>;
}

impl ExtractProjectInformation for Path {
    fn extract_version(&self) -> anyhow::Result<&str> {
        let pattern = get_filename_pattern();
        let filename = self.extract_filename();
        let captures = pattern.captures(filename).ok_or_else(|| {
            anyhow::anyhow!(
                "Filename {} not in expected `<project>-<version>` format", filename
            )
        })?;
        let version = captures.name("version")
            .expect("Version group not found")
            .as_str();
        Ok(version)
    }

    fn extract_project(&self) -> anyhow::Result<&str> {
        let pattern = get_filename_pattern();
        let filename = self.extract_filename();
        let captures = pattern.captures(filename).ok_or_else(|| {
            anyhow::anyhow!(
                "Filename {} not in expected `<project>-<version>` format", filename
            )
        })?;
        let version = captures.name("project")
            .expect("Version group not found")
            .as_str();
        Ok(version)
    }
}

pub fn cmp_versions(a: &str, b: &str) -> Ordering {
    let lhs = a.split('.');
    let rhs = b.split('.');
    for pair in lhs.zip_longest(rhs) {
        match pair {
            EitherOrBoth::Both(x, y) => {
                let p = x.parse::<u64>();
                let q = y.parse::<u64>();
                match (p, q) {
                    (Ok(u), Ok(v)) if u < v => { return Ordering::Less; }
                    (Ok(u), Ok(v)) if u > v => { return Ordering::Greater; }
                    (Ok(_), Err(_)) => { return Ordering::Greater; }
                    (Err(_), Ok(_)) => { return Ordering::Less; }
                    (Err(_), Err(_)) => { 
                        let c = x.cmp(y);
                        if c != Ordering::Equal {
                            return c;
                        }
                    }
                    _ => {}
                }
            }
            EitherOrBoth::Left(_) => {
                return Ordering::Greater;
            }
            EitherOrBoth::Right(_) => {
                return Ordering::Less;
            }
        }
    }
    Ordering::Equal
}