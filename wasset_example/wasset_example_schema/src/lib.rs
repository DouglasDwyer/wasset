use serde::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExampleAsset {
    Binary(Vec<u8>),
    Text(String)
}