#[derive(Debug, Clone, serde::Deserialize)]
pub struct Record {
    #[serde(rename = "class1")] pub from: String,
    #[serde(rename = "class2")] pub to: String,
    #[serde(rename = "comments#Cosine")] pub cosine_1: f64,
    #[serde(rename = "imports#Cosine")] pub cosine_2: f64,
    #[serde(rename = "methods#Cosine")] pub cosine_3: f64,
    #[serde(rename = "variables#Cosine")] pub cosine_4: f64,
    #[serde(rename = "fields#Cosine")] pub cosine_5: f64,
    #[serde(rename = "calls#Cosine")] pub cosine_6: f64,
    #[serde(rename = "imports-fields-methods-variables-comments#Cosine")] pub cosine_7: f64,
    #[serde(rename = "imports-fields-methods-variables#Cosine")] pub cosine_8: f64,
    #[serde(rename = "fields-variables-methods#Cosine")] pub cosine_9: f64,
    #[serde(rename = "fields-methods#Cosine")] pub cosine_10: f64,
    #[serde(rename = "fields-variables#Cosine")] pub cosine_11: f64,
    #[serde(rename = "imports-fields-methods-variables-comments-calls#Cosine")] pub cosine_12: f64,
    #[serde(rename = "imports-fields-methods-variables-calls#Cosine")] pub cosine_13: f64,
    #[serde(rename = "fields-variables-methods-calls#Cosine")] pub cosine_14: f64,
    #[serde(rename = "fields-methods-calls#Cosine")] pub cosine_15: f64,
    #[serde(rename = "methods-calls#Cosine")] pub cosine_16: f64,
    #[serde(rename = "")] null: String 
}
