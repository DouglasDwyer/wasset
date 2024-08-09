//! # wasset
//! 
//! This crate allows for embedding external asset files (text, images, models) into WASM plugins. The assets are stored in the WASM module's custom data section. This allows for reading assets on the host using a `WassetParser`. The WASM module itself can reference its assets by macro-generated ID.
//! 
//! `wasset` is meant to be a foundation for game asset systems. It differs from using `include_bytes!()` to include assets in the following ways:
//! 
//! - The host can read a WASM module's assets without loading the WASM itself. This allows a game engine to preload or lazily load all assets, before instantiating the WASM modules.
//! - Separate WASM modules can reference and share assets by `WassetId`.
//! - Because assets are stored in a custom section, it's not necessary to load all assets into memory when instantiating the WASM module. This can conserve memory for WASM modules that include many assets.
//! 
//! ---
//! 
//! ## Usage
//! 
//! `wasset` does not define an asset format - rather, it provides the means to load and store a user-specified asset type to WASM. Therefore, setting up `wasset` requires:
//! 
//! - Defining an `AssetSchema` type that can be serialized and deserialized with `serde`
//! - Implementing `AssetEncoder` to determine how files are turned into assets
//! - Re-exporting the `wasset::include_assets::<A: AssetEncoder>(path)` macro with the appropriate asset encoder type
//! 
//! [A complete example is available here.](/wasset_example/) Once the asset type and macro have been defined, they may be used from within WASM as follows:
//! 
//! ```ignore
//! use wasset::*;
//! use wasset_example_macro::*;
//! 
//! // Load all assets from the given folder. This is the
//! // macro defined for a specific `AssetEncoder`.
//! include_assets!("wasset_example/wasset_example_module/assets");
//! 
//! /// Gets a list of all assets from this module.
//! pub fn list_all_assets() -> &'static [WassetId] {
//!     &[
//!         // The macro has defined these for us.
//!         assets::SOME_BINARY,
//!         assets::SOME_TEXT,
//!         assets::submodule::MORE_TEXT
//!     ]
//! }
//! ```
//! 
//! Then, the asset data for this WASM plugin may be examined from the host:
//! 
//! ```ignore
//! use wasset::*;
//! use wasset_example_schema::*;
//! 
//! fn main() {
//!     // Create a parser from the bytes of the WASM module.
//!     let parser = WassetParser::<ExampleAsset>::parse(EXAMPLE_PLUGIN_WASM).unwrap();
//!     for (id, asset) in &parser {
//!         println!("{id:?} | {asset:?}");
//! 
//!         // Prints something like:
//!         // WassetId(9ee64711-8e7e-4e40-a1bc-f13a1b4e5bdb) | Ok(Binary([97, 115, 106, 100]))
//!         // WassetId(b230fb86-8bf6-49a0-94f9-624386204129) | Ok(Text("Even more!"))
//!         // WassetId(ae189ff9-b0d4-48fc-b0e1-3093d53bff85) | Ok(Text("Hello world!"))
//!     }
//! }
//! ```
//! 
//! ## Optional features
//! 
//! - **bytemuck** - implements the `Pod` and `Zeroable` attributes on relevant types.
//! - **encode** - allows for serializing a folder of assets into memory.
//! - **encode_macro** - exposes a generic macro that, when instantiated, will embed a folder of assets into a WASM module.
//! - **parse** - exposes the ability to read a WASM module's assets.
//! - **relative_path** - (requires nightly) makes the `encode_macro` use relative paths rather than paths from the project root.

#![deny(warnings)]
#![warn(clippy::missing_docs_in_private_items)]

#![cfg_attr(all(unstable, feature = "encode_macro"), feature(track_path))]
#![cfg_attr(feature = "relative_path", feature(proc_macro_span))]

#[cfg(feature = "encode")]
pub use crate::encode::*;

#[cfg(feature = "parse")]
pub use crate::parse::*;

#[cfg(feature = "bytemuck")]
use bytemuck::*;
use fxhash::*;
use ::serde::*;
use std::borrow::*;
use std::ops::*;
use uuid::*;

#[cfg(feature = "encode")]
/// Implements the ability to write assets from a directory.
mod encode;

#[cfg(feature = "parse")]
/// Implements the ability to read assets from a WASM module.
mod parse;

/// Represents an asset type which may be stored and loaded from WASM.
pub trait AssetSchema: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de> {}

impl<T: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>> AssetSchema for T {}

/// A unique ID associated with an asset.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WassetId(Uuid);

impl WassetId {
    /// Creates a new ID from the given group of bytes.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Gets a representation of this ID as bytes.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl From<Uuid> for WassetId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<WassetId> for Uuid {
    fn from(value: WassetId) -> Self {
        value.0
    }
}

impl Borrow<[u8; 16]> for WassetId {
    fn borrow(&self) -> &[u8; 16] {
        self.as_bytes()
    }
}

#[cfg(feature = "bytemuck")]
unsafe impl Pod for WassetId {}

#[cfg(feature = "bytemuck")]
unsafe impl Zeroable for WassetId {}

/// A list which describes the list of assets present in a WASM module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WassetManifest {
    /// A mapping from asset IDs to offsets within a custom data section.
    asset_ranges: FxHashMap<WassetId, Range<u32>>
}

impl WassetManifest {
    /// Gets an iterator over the IDs of all assets stored in the module.
    pub fn ids(&self) -> impl '_ + Iterator<Item = WassetId> {
        self.asset_ranges.keys().copied()
    }
}

/// Represents an error that occurred during asset processing.
#[derive(Debug, thiserror::Error)]
pub enum WassetError {
    /// An error was raised while writing assets.
    #[error("An error occurred during serialization: {0}")]
    Serialize(Box<dyn std::error::Error + Send + Sync>),
    /// An error was raised while reading assets.
    #[error("An error occurred during deserialization: {0}")]
    Deserialize(Box<dyn std::error::Error + Send + Sync>)
}

impl WassetError {
    /// Creates a new `Self::Deserialize` error with the given contents.
    pub fn from_deserialize(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Deserialize(err.into())
    }

    /// Creates a new `Self::Serialize` error with the given contents.
    pub fn from_serialize(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Serialize(err.into())
    }
}