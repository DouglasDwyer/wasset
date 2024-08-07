use wasset::*;
use wasset_example_macro::*;

/// Load all assets from the given folder.
include_assets!("wasset_example/wasset_example_module/assets");

/// Gets a list of all assets from this module.
pub fn list_all_assets() -> &'static [WassetId] {
    &[
        /// The macro has defined these for us.
        assets::SOME_BINARY,
        assets::SOME_TEXT,
        assets::submodule::MORE_TEXT
    ]
}