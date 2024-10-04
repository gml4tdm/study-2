
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DependencyGraph {
    #[allow(unused)] pub header: Header,
    pub context: Context
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Header
////////////////////////////////////////////////////////////////////////////////

#[allow(unused)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Header {
    #[serde(rename = "created-by")]
    pub created_by: CreatedBy
}

#[allow(unused)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreatedBy {
    pub exporter: Exporter,
    pub provider: Provider
}

#[allow(unused)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Exporter {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "$value")]
    pub name: String
}

#[allow(unused)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Provider {
    #[serde(rename = "$value")]
    pub name: String
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Context
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Context {
    #[allow(unused)]
    #[serde(rename = "@name")]
    pub name: String,
    
    #[serde(default)]
    pub containers: Vec<Container>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Container {
    #[allow(unused)]
    #[serde(rename = "@name")]
    pub name: String,
    
    #[allow(unused)]
    #[serde(rename = "@classification")]
    pub classification: ContainerClassification,
    
    #[serde(default)]
    pub namespaces: Vec<Namespace>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Namespace {
    #[serde(rename = "@name")]
    pub name: String,
    
    pub types: Vec<Type>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Type {
    #[serde(rename = "@name")]
    pub name: String,
    
    #[allow(unused)]
    #[serde(rename = "@visibility")]
    pub visibility: Visibility,
    
    #[serde(rename = "@classification")]
    pub classification: TypeClassification,
    pub dependencies: Dependencies
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Dependencies {
    #[allow(unused)]
    #[serde(rename = "@count")]
    pub count: i32,
    
    #[serde(rename = "depends-on", default)]
    pub dependencies: Vec<DependsOn>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DependsOn {
    #[serde(rename = "@name")]
    pub name: String,
    #[allow(unused)]
    #[serde(rename = "@classification")]
    pub classification: DependencyClassification
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
// Lexical Elements
////////////////////////////////////////////////////////////////////////////////

#[allow(unused)]
#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum ContainerClassification {
    #[serde(rename = "jar")] Jar
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum Visibility {
    #[serde(rename = "public")] Public,
    #[serde(rename = "private")] Private,
    #[serde(rename = "protected")] Protected,
    #[serde(rename = "default")] Default
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum TypeClassification {
    #[serde(rename = "class")] Class,
    #[serde(rename = "interface")] Interface,
    #[serde(rename = "enum")] Enum,
    #[serde(rename = "annotation")] Annotation
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum DependencyClassification {
    #[serde(rename = "uses")] Uses,
    #[serde(rename = "extends")] Extends,
    #[serde(rename = "implements")] Implements,
}
