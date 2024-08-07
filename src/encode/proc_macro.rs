// Reference the proc-macro crate, as this is a normal binary.
extern crate proc_macro;

use crate::*;
use litrs::*;
use proc_macro::*;
use quote::quote;
use std::fs::*;
use std::path::*;

/// Provides a macro implementation which accepts a directory path and outputs
/// code which embeds all assets in the directory. This should be called with a concrete
/// asset type from a user-defined macro.
pub fn include_assets<A: AssetEncoder>(x: TokenStream) -> TokenStream {
    let input = x.into_iter().map(Into::into).collect::<Vec<TokenTree>>();
    assert!(input.len() == 1, "Wrong number of arguments.");
    let x = StringLit::try_from(&input[0])
        .expect("Could not parse argument as path string.")
        .into_value();

    #[allow(unused)]
    let mut parent_dir_path = None;
    #[cfg(feature = "relative_path")]
    {
        let mut path = Span::call_site().source_file().path();
        path.pop();
        parent_dir_path = Some(path);
    }

    let resolved_path = resolve_path(&x, parent_dir_path).expect("Could not resolve path.");
    
    #[cfg(unstable)]
    tracked_path::path(resolved_path.display().to_string());

    let assets = encode_asset_folder::<A>(&resolved_path).expect("Failed to encode assets");
    write_assets(&assets)
}

/// Writes the set of encoded assets as code.
fn write_assets(assets: &EncodedAssets) -> TokenStream {
    let id = Uuid::new_v4();
    let manifest_name = proc_macro2::Literal::string(&format!("__wasset_manifest:{id}"));
    let contents_name = proc_macro2::Literal::string(&format!("__wasset_data:{id}"));

    let manifest_literal_len = proc_macro2::Literal::usize_unsuffixed(assets.manifest.len());
    let manifest_literal = proc_macro2::Literal::byte_string(&assets.manifest);
    let contents_literal_len = proc_macro2::Literal::usize_unsuffixed(assets.data.len());
    let contents_literal = proc_macro2::Literal::byte_string(&assets.data);

    let mut data = quote! {
        const _: () = {
            #[link_section = #manifest_name]
            static ASSET_MANIFEST: [u8; #manifest_literal_len] = *#manifest_literal;
    
            #[link_section = #contents_name]
            static ASSET_DATA: [u8; #contents_literal_len] = *#contents_literal;
        };
    };

    data.extend(assets.encoded_assets.iter().map(|(name, hierarchy)| tokens_for_hierarchy(name, hierarchy)));

    data.into()
}

/// Canonicalizes the path provided by the user.
fn resolve_path(path: &str, parent_dir_path: Option<PathBuf>) -> std::io::Result<PathBuf> {
    let mut path = PathBuf::from(path);
    if let Some(p) = parent_dir_path {
        if !path.is_absolute() {
            path = p.join(path);
        }
    }
    canonicalize(&path)
}

/// Gets tokens which encode the given asset hierarchy.
fn tokens_for_hierarchy(name: &str, hierarchy: &AssetHierarchy) -> proc_macro2::TokenStream {
    let mut inner_module = proc_macro2::TokenStream::new();
    inner_module.extend(hierarchy.sub_hierarchies.iter().map(|(n, h)| tokens_for_hierarchy(n, h)));
    inner_module.extend(hierarchy.assets.iter().map(|entry| {
        let entry_name = proc_macro2::Ident::new(&entry.name.to_uppercase(), proc_macro2::Span::call_site());
        let byte_data = proc_macro2::Literal::byte_string(&entry.id.as_bytes()[..]);

        quote! {
            pub const #entry_name: ::wasset::WassetId = ::wasset::WassetId::from_bytes(* #byte_data);
        }
    }));

    let module_name = proc_macro2::Ident::new(name, proc_macro2::Span::call_site());

    quote! {
        pub mod #module_name {
            #inner_module
        }
    }
}