#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphFeatureData {
    pub nodes: Vec<String>,
    pub edges: Vec<Edge>,
    pub pairs_without_semantic_features: Vec<Edge>,
    pub pairs_without_topological_features: Vec<Edge>,
    pub link_features: Vec<LinkFeature>
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LinkFeature {
    pub edge: Edge,
    pub common_neighbours: i32,
    pub salton: f64,
    pub sorenson: f64,
    pub adamic_adar: f64,
    pub russel_rao: f64,
    pub resource_allocation: f64,
    pub katz: f64,
    pub sim_rank: f64,
    pub cosine_1: f64,
    pub cosine_2: f64,
    pub cosine_3: f64,
    pub cosine_4: f64,
    pub cosine_5: f64,
    pub cosine_6: f64,
    pub cosine_7: f64,
    pub cosine_8: f64,
    pub cosine_9: f64,
    pub cosine_10: f64,
    pub cosine_11: f64,
    pub cosine_12: f64,
    pub cosine_13: f64,
    pub cosine_14: f64,
    pub cosine_15: f64,
    pub cosine_16: f64
}