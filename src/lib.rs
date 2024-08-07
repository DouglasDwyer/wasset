#![cfg_attr(all(unstable, feature = "encode_macro"), feature(track_path))]
#![cfg_attr(feature = "relative_path", feature(proc_macro_span))]

#[cfg(feature = "encode")]
pub use crate::encode::*;

#[cfg(feature = "parse")]
pub use crate::parse::*;

use fxhash::*;
use ::serde::*;
use std::ops::*;
use uuid::*;

#[cfg(feature = "encode")]
mod encode;

#[cfg(feature = "parse")]
mod parse;

pub trait AssetSchema: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de> {}

impl<T: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>> AssetSchema for T {}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WassetId(Uuid);

impl WassetId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WassetManifest {
    asset_ranges: FxHashMap<WassetId, Range<u32>>
}

#[derive(Debug, thiserror::Error)]
pub enum WassetError {
    #[error("An error occurred during serialization: {0}")]
    Serialize(Box<dyn std::error::Error + Send + Sync>),
    #[error("An error occurred during deserialization: {0}")]
    Deserialize(Box<dyn std::error::Error + Send + Sync>)
}

impl WassetError {
    pub fn from_deserialize(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        //panic!("WTF");
        Self::Deserialize(err.into())
    }

    pub fn from_serialize(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Serialize(err.into())
    }
}