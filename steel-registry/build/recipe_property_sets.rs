use std::{collections::BTreeMap, fs};

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

#[must_use]
fn to_item_ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

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
            (
                Ident::new(&key.to_uppercase(), Span::call_site()),
                list.iter()
                    .map(|it| to_item_ident(it.strip_prefix("minecraft:").unwrap_or(it)))
                    .map(|it| quote! { &ITEMS.#it, }),
            )
        })
        .map(|(key, list)| {
            quote! {
                pub const #key: LazyLock<&'static [ItemRef]> = LazyLock::new(|| Box::leak(vec![#(#list)*].into_boxed_slice()));
            }
        })
        .collect();

    quote! {
        use crate::{items::ItemRef, vanilla_items::ITEMS};
        use std::sync::LazyLock;
        #(#recipe_property_sets)*
    }
}
