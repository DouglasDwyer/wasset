# wasset

[![Crates.io](https://img.shields.io/crates/v/wasset.svg)](https://crates.io/crates/wasset)
[![Docs.rs](https://docs.rs/wasset/badge.svg)](https://docs.rs/wasset)

This crate allows for embedding external asset files (text, images, models) into WASM plugins. The assets are stored in the WASM module's custom data section. This allows for reading assets on the host using a `WassetParser`. The WASM module itself can reference its assets by macro-generated ID.

`wasset` is meant to be a foundation for game asset systems. It differs from using `include_bytes!()` to include assets in the following ways:

- The host can read a WASM module's assets without loading the WASM itself. This allows a game engine to preload or lazily load all assets, before instantiating the WASM modules.
- Separate WASM modules can reference and share assets by `WassetId`.
- Because assets are stored in a custom section, it's not necessary to load all assets into memory when instantiating the WASM module. This can conserve memory for WASM modules that include many assets.

---

## Usage

`wasset` does not define an asset format - rather, it provides the means to load and store a user-specified asset type to WASM. Therefore, setting up `wasset` requires:

- Defining an `AssetSchema` type that can be serialized and deserialized with `serde`
- Implementing `AssetEncoder` to determine how files are turned into assets
- Re-exporting the `wasset::include_assets::<A: AssetEncoder>(path)` macro with the appropriate asset encoder type

[A complete example is available here.](/wasset_example/) Once the asset type and macro have been defined, they may be used from within WASM as follows:

```rust
use wasset::*;
use wasset_example_macro::*;

// Load all assets from the given folder. This is the
// macro defined for a specific `AssetEncoder`.
include_assets!("wasset_example/wasset_example_module/assets");

/// Gets a list of all assets from this module.
pub fn list_all_assets() -> &'static [WassetId] {
    &[
        // The macro has defined these for us.
        assets::SOME_BINARY,
        assets::SOME_TEXT,
        assets::submodule::MORE_TEXT
    ]
}
```

Then, the asset data for this WASM plugin may be examined from the host:

```rust
use wasset::*;
use wasset_example_schema::*;

fn main() {
    // Create a parser from the bytes of the WASM module.
    let parser = WassetParser::<ExampleAsset>::parse(EXAMPLE_PLUGIN_WASM).unwrap();
    for (id, asset) in &parser {
        println!("{id:?} | {asset:?}");

        // Prints something like:
        // WassetId(9ee64711-8e7e-4e40-a1bc-f13a1b4e5bdb) | Ok(Binary([97, 115, 106, 100]))
        // WassetId(b230fb86-8bf6-49a0-94f9-624386204129) | Ok(Text("Even more!"))
        // WassetId(ae189ff9-b0d4-48fc-b0e1-3093d53bff85) | Ok(Text("Hello world!"))
    }
}
```

## Optional features

- **encode** - allows for serializing a folder of assets into memory.
- **encode_macro** - exposes a generic macro that, when instantiated, will embed a folder of assets into a WASM module.
- **parse** - exposes the ability to read a WASM module's assets.
- **relative_path** - (requires nightly) makes the `encode_macro` use relative paths rather than paths from the project root.