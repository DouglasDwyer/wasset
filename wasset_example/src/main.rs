use wasset::*;
use wasset_example_schema::*;

include!(concat!(env!("OUT_DIR"), "/example_plugin.rs"));

fn main() {
    let parser = WassetParser::<ExampleAsset>::parse(EXAMPLE_PLUGIN_WASM).unwrap();
    for (id, asset) in &parser {
        println!("{id:?} | {asset:?}");

        // Prints something like:
        // WassetId(9ee64711-8e7e-4e40-a1bc-f13a1b4e5bdb) | Ok(Binary([97, 115, 106, 100]))
        // WassetId(b230fb86-8bf6-49a0-94f9-624386204129) | Ok(Text("Even more!"))
        // WassetId(ae189ff9-b0d4-48fc-b0e1-3093d53bff85) | Ok(Text("Hello world!"))
    }
}
