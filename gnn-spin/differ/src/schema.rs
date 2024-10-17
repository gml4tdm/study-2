////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Top-level graph
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename = "ODEM")]
pub struct DependencyGraphRoot {
    pub header: Header,
    pub context: Context
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Header
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Header {
    #[serde(rename = "created-by")] pub created_by: CreatedBy
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreatedBy {
    pub exporter: Exporter,
    pub provider: Provider
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Exporter {
    #[serde(rename = "@version")] pub version: String,
    #[serde(rename = "$value")] pub name: String
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Provider {
    #[serde(rename = "$value")] pub name: String
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Actual Graph
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Context {
    #[serde(rename = "@name")] pub name: String,
    #[serde(rename = "container", default)] pub containers: Vec<Container>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Container {
    #[serde(rename = "@name")] pub name: String,
    #[serde(rename = "namespace", default)] pub namespaces: Vec<Namespace>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Namespace {
    #[serde(rename = "@name")] pub name: String,
    #[serde(rename = "type", default)] pub types: Vec<Type>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Type {
    #[serde(rename = "@name")] pub name: String,
    #[serde(rename = "@classification")] pub classification: TypeClassification,
    #[serde(rename = "@visibility")] pub visibility: Visibility,
    pub dependencies: Dependencies 
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Dependencies {
    #[serde(rename = "@count")] pub count: i32,
    #[serde(rename = "depends-on", default)] pub depends_on: Vec<DependsOn>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DependsOn {
    #[serde(rename = "@name")] pub name: String,
    #[serde(rename = "@classification")] pub classification: DependsOnClassification,
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Enums
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum TypeClassification {
    #[serde(rename = "class")] Class,
    #[serde(rename = "interface")] Interface,
    #[serde(rename = "enum")] Enum,
    #[serde(rename = "struct")] Struct,
    #[serde(rename = "annotation")] Annotation,
    #[serde(rename = "unknown")] Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum Visibility {
    #[serde(rename = "public")] Public,
    #[serde(rename = "protected")] Protected,
    #[serde(rename = "private")] Private,
    #[serde(rename = "default")] Default
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum DependsOnClassification {
    #[serde(rename = "uses")] Uses,
    #[serde(rename = "extends")] Extends,
    #[serde(rename = "implements")] Implements,
}
