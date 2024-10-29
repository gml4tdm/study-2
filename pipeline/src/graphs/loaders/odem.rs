use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::graphs::{DependencyGraph, DependencySpec, DependencyType};

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Top-level graph
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename = "ODEM")]
pub struct OdemGraphRoot {
    pub header: Header,
    pub context: Context,
}

impl OdemGraphRoot {
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let graph = quick_xml::de::from_reader(reader)?;
        Ok(graph)
    }
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Header
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Header {
    #[serde(rename = "created-by")]
    pub created_by: CreatedBy,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreatedBy {
    pub exporter: Exporter,
    pub provider: Provider,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Exporter {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "$value")]
    pub name: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Provider {
    #[serde(rename = "$value")]
    pub name: String,
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Actual Graph
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Context {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "container", default)]
    pub containers: Vec<Container>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Container {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "namespace", default)]
    pub namespaces: Vec<Namespace>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Namespace {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "type", default)]
    pub types: Vec<Type>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Type {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@classification")]
    pub classification: TypeClassification,
    #[serde(rename = "@visibility")]
    pub visibility: Visibility,
    pub dependencies: Dependencies,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Dependencies {
    #[serde(rename = "@count")]
    pub count: i32,
    #[serde(rename = "depends-on", default)]
    pub depends_on: Vec<DependsOn>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DependsOn {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@classification")]
    pub classification: DependsOnClassification,
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Enums
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum TypeClassification {
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "interface")]
    Interface,
    #[serde(rename = "enum")]
    Enum,
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "annotation")]
    Annotation,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum Visibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "protected")]
    Protected,
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "default")]
    Default,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum DependsOnClassification {
    #[serde(rename = "uses")]
    Uses,
    #[serde(rename = "extends")]
    Extends,
    #[serde(rename = "implements")]
    Implements,
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Conversion to Generic Graph
////////////////////////////////////////////////////////////////////////////////


impl From<OdemGraphRoot> for DependencyGraph {
    fn from(root: OdemGraphRoot) -> Self {
        let mut nodes = HashSet::new();
        let mut edges = HashMap::new();

        for container in root.context.containers {
            for namespace in container.namespaces {
                for r#type in namespace.types {
                    nodes.insert(r#type.name.clone());
                    for depends_on in r#type.dependencies.depends_on {
                        let key = (r#type.name.clone(), depends_on.name.clone());
                        let value = match depends_on.classification {
                            DependsOnClassification::Uses => DependencyType::Uses,
                            DependsOnClassification::Extends => DependencyType::Extends,
                            DependsOnClassification::Implements => DependencyType::Implements,
                        };
                        edges.entry(key)
                            .or_insert(DependencySpec::default())
                            .increment(value);
                    }
                }
            }
        }

        DependencyGraph::new(nodes, edges)
    }
}
