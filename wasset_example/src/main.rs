use wasset::*;
use wasset_example_schema::*;

include!(concat!(env!("OUT_DIR"), "/example_plugin.rs"));

fn main() {
    let parser = WassetParser::<ExampleAsset>::parse(EXAMPLE_PLUGIN_WASM).unwrap();
    for (id, asset) in &parser {
        println!("{id:?} | {asset:?}");
    }
}
