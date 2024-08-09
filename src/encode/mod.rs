use crate::*;
pub use crate::encode::proc_macro::*;
use std::fs::*;
use std::path::*;
use toml::*;

/// Defines macros for easily embedding assets.
mod proc_macro;

/// Represents a type that can load assets from files on disk.
pub trait AssetEncoder {
    /// The target asset type that this encoder produces.
    type Target: AssetSchema;

    /// Creates a new `Target` asset from file data. The target asset data may be modified
    /// based upon the file `extension`, or by the `metadata` from a `Wasset.toml` file
    /// in the same directory.
    fn encode(extension: &str, metadata: &Table, data: Vec<u8>) -> Result<Option<Self::Target>, WassetError>;
}

/// Denotes an asset that has been serialized.
#[derive(Clone, Debug)]
pub struct EncodedAsset {
    /// The name of the asset that should be displayed to the developer.
    pub name: String,
    /// The asset ID.
    pub id: WassetId
}

/// Represents a hierarchy of assets that have been serialized.
#[derive(Clone, Debug, Default)]
pub struct AssetHierarchy {
    /// The assets on this level of the hierarchy.
    pub assets: Vec<EncodedAsset>,
    /// A mapping from display names to subhierarchies.
    pub sub_hierarchies: FxHashMap<String, AssetHierarchy>
}

/// Holds an entire set of assets that have been serialized from files on disk.
#[derive(Debug, Default)]
pub struct EncodedAssets {
    /// The data that should be written to the custom section for holding the assets.
    pub data: Vec<u8>,
    /// The encoded asset names and IDs, so that they may be referenced by the WASM plugin.
    pub encoded_assets: FxHashMap<String, AssetHierarchy>,
    /// The serialized manifest describing the assets.
    pub manifest: Vec<u8>,
}

/// Loads all assets from the provided folder into an `EncodedAssets` structure.
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

/// Represents an ongoing operation to encode assets.
struct EncodingOperation<'a> {
    /// The data section.
    pub data: &'a mut Vec<u8>,
    /// The current hierarchy level.
    pub encoded_assets: &'a mut AssetHierarchy,
    /// The manifest.
    pub manifest: &'a mut WassetManifest
}

/// Loads all assets from a certain folder into the `operation`.
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

                if let Some(asset) = A::encode(&path.extension().unwrap_or_default().to_string_lossy(), metadata, read(&path).map_err(WassetError::from_serialize)?)? {
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

/// Gets the name at the end of the file path as a string.
fn name_for_path(path: &Path) -> Result<Cow<str>, WassetError> {
    Ok(path.file_name().ok_or_else(|| WassetError::from_serialize("Failed to get file system name"))?.to_string_lossy())
}