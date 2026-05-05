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

    let mut statics = TokenStream::new();
    let mut registrations = TokenStream::new();

    recipe_property_sets_entries
        .iter()
        .for_each(|(ident, list)| {
            let key = Ident::new(&ident.to_snake_case().to_uppercase(), Span::call_site());
            let items = list.iter().map(|it| {
                let ident = Ident::new(
                    it.strip_prefix("minecraft:").unwrap_or(it),
                    Span::call_site(),
                );
                quote! { &ITEMS.#ident }
            });

            statics.extend(quote! {
                pub static #key: RecipePropertySet = RecipePropertySet {
                    key: Identifier::vanilla_static(#ident),
                    items: OnceLock::new()
                };
            });
            registrations
                .extend(quote! { registry.register_with_items(&#key, vec![#(#items),*]); registry.register(&#key); });
        });

    quote! {
        use steel_utils::Identifier;
        use crate::{items::ItemRef, recipe::{RecipePropertySet, RecipePropertySetRegistry}, vanilla_items::ITEMS};
        use std::sync::OnceLock;

        #statics

        pub fn register_recipe_property_sets(registry: &mut RecipePropertySetRegistry) {
            #registrations
        }
    }
}
