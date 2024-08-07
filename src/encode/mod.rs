use crate::*;
pub use crate::encode::proc_macro::*;
use std::borrow::*;
use std::fs::*;
use std::path::*;
use toml::*;

mod proc_macro;

pub trait AssetEncoder {
    type Target: AssetSchema;

    fn encode(extension: &str, metadata: &Table, data: Vec<u8>) -> Result<Option<Self::Target>, WassetError>;
}

#[derive(Clone, Debug)]
pub struct EncodedAsset {
    pub name: String,
    pub id: WassetId
}

#[derive(Clone, Debug, Default)]
pub struct AssetHierarchy {
    pub assets: Vec<EncodedAsset>,
    pub sub_hierarchies: FxHashMap<String, AssetHierarchy>
}

#[derive(Debug, Default)]
pub struct EncodedAssets {
    pub data: Vec<u8>,
    pub encoded_assets: FxHashMap<String, AssetHierarchy>,
    pub manifest: Vec<u8>,
}

pub fn encode_asset_folder<A: AssetEncoder>(folder: &Path) -> Result<EncodedAssets, WassetError> {
    let mut data = Vec::new();
    let mut hierarchy = AssetHierarchy::default();
    let mut manifest = WassetManifest::default();

    let base = folder.parent().ok_or_else(|| WassetError::from_serialize("Folder must have name."))?;
    load_assets_in_folder::<A>(base, folder, &mut EncodingOperation {
        data: &mut data,
        encoded_assets: &mut hierarchy,
        manifest: &mut manifest
    })?;

    let name = name_for_path(folder)?;
    let encoded_assets = FxHashMap::from_iter([(name.into_owned(), hierarchy)]);

    Ok(EncodedAssets {
        data,
        encoded_assets,
        manifest: rmp_serde::to_vec_named(&manifest).map_err(WassetError::from_serialize)?
    })
}

struct EncodingOperation<'a> {
    pub data: &'a mut Vec<u8>,
    pub encoded_assets: &'a mut AssetHierarchy,
    pub manifest: &'a mut WassetManifest
}

fn load_assets_in_folder<A: AssetEncoder>(base: &Path, folder: &Path, operation: &mut EncodingOperation) -> Result<(), WassetError> {
    let master_table = if let Ok(options) = read_to_string(folder.join("Wasset.toml")) {
        options.parse::<Table>().map_err(WassetError::from_serialize)?
    }
    else {
        Table::default()
    };

    for to_read in read_dir(folder).map_err(WassetError::from_serialize)? {
        let entry = to_read.map_err(WassetError::from_serialize)?;
        let path = entry.path();
        if path.is_dir() {
            let entry_name = name_for_path(&path)?;
            load_assets_in_folder::<A>(base, &path, &mut EncodingOperation {
                data: operation.data,
                encoded_assets: operation.encoded_assets.sub_hierarchies.entry(entry_name.into_owned()).or_default(),
                manifest: operation.manifest
            })?;
        }
        else if path.is_file() {
            let asset_path = path.strip_prefix(base).ok().map(|x| x.with_extension(""));
            if let Some(local_path) = asset_path {
                let default_table = Table::default();
                let file_name = name_for_path(&path)?;
                let metadata = match master_table.get(&*file_name) {
                    Some(Value::Table(x)) => x,
                    None => &default_table,
                    Some(x) => return Err(WassetError::from_serialize(format!("Unexpected metadata value {x:?} for asset {file_name}; expected table")))
                };

                if let Some(asset) = A::encode(&path.extension().unwrap_or_default().to_string_lossy(), &metadata, read(&path).map_err(WassetError::from_serialize)?)? {
                    let entry_name = name_for_path(&local_path)?;
                    let id = WassetId::from(Uuid::new_v4());

                    let start = operation.data.len() as u32;
                    rmp_serde::encode::write_named(operation.data, &asset).map_err(WassetError::from_serialize)?;
                    let end = operation.data.len() as u32;
                    operation.manifest.asset_ranges.insert(id, start..end);
                    operation.encoded_assets.assets.push(EncodedAsset {
                        name: entry_name.into_owned(),
                        id
                    })
                }
            }
        }
    }

    Ok(())
}

fn name_for_path<'a>(path: &'a Path) -> Result<Cow<'a, str>, WassetError> {
    Ok(path.file_name().ok_or_else(|| WassetError::from_serialize("Failed to get file system name"))?.to_string_lossy())
}