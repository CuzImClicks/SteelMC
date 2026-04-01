use std::{collections::BTreeMap, fs};

use heck::ToSnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/recipe_property_sets.json");

    let recipe_property_sets_json = fs::read_to_string("build_assets/recipe_property_sets.json")
        .expect("Failed to read recipe_property_sets.json");
    let recipe_property_sets_entries: BTreeMap<String, Vec<String>> =
        serde_json::from_str(&recipe_property_sets_json)
            .expect("Failed to parse recipe_property_sets.json");

    let recipe_property_sets: Vec<TokenStream> = recipe_property_sets_entries
        .iter()
        .map(|(key, list)| {
            let key = Ident::new(&key.to_snake_case().to_uppercase(), Span::call_site());
            let items = list.iter().map(|it| {
                let ident = Ident::new(it.strip_prefix("minecraft:").unwrap_or(it), Span::call_site());
                quote! { &ITEMS.#ident }
            });
            quote! {
                pub static #key: LazyLock<&'static [ItemRef]> = LazyLock::new(|| Box::leak(vec![#(#items),*].into_boxed_slice()));
            }
        })
        .collect();

    quote! {
        use crate::{items::ItemRef, vanilla_items::ITEMS};
        use std::sync::LazyLock;
        #(#recipe_property_sets)*
    }
}
