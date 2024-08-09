extern crate proc_macro;
use proc_macro::*;
use toml::*;
use wasset::*;
use wasset_example_schema::*;

#[proc_macro]
pub fn include_assets(path: TokenStream) -> TokenStream {
    wasset::include_assets::<ExampleAssetEncoder>(path, &quote::quote! { ::wasset::WassetId })
}

struct ExampleAssetEncoder;

impl AssetEncoder for ExampleAssetEncoder {
    type Target = ExampleAsset;

    fn encode(extension: &str, metadata: &Table, data: Vec<u8>) -> Result<Option<Self::Target>, WassetError> {
        match extension {
            "txt" => {
                let mut data = String::from_utf8_lossy(&data).into_owned();
                match metadata.get("append") {
                    Some(Value::String(x)) => data.extend(x.chars()),
                    Some(x) => return Err(WassetError::from_serialize(format!("Unexpected metadata value {x:?}"))),
                    None => {},
                }

                Ok(Some(ExampleAsset::Text(data)))
            },
            "bin" => Ok(Some(ExampleAsset::Binary(data))),
            _ => Ok(None)
        }
    }
}