use crate::generator_functions::{
    generate_identifier, generate_option, generate_text_component, read_variants_from_dir,
};
use crate::shared_structs::TextComponentJson;
use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use steel_utils::Identifier;

#[derive(Deserialize, Debug)]
pub struct PaintingVariantJson {
    width: i32,
    height: i32,
    asset_id: Identifier,
    #[serde(default)]
    title: Option<TextComponentJson>,
    #[serde(default)]
    author: Option<TextComponentJson>,
}

pub(crate) fn build() -> TokenStream {
    let painting_variants: Vec<(String, PaintingVariantJson)> =
        read_variants_from_dir("painting_variant");

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::painting_variant::{
            PaintingVariant, PaintingVariantRegistry, PaintingVariantValue,
        };
        use steel_utils::Identifier;
        use text_components::{
            TextComponent, content::Content, format::{Color, Format}, interactivity::Interactivity, translation::TranslatedMessage
        };
        use std::borrow::Cow;
    });

    // Generate static painting variant definitions
    let mut register_stream = TokenStream::new();
    for (painting_variant_name, painting_variant) in &painting_variants {
        let painting_variant_ident = Ident::new(
            &painting_variant_name.to_shouty_snake_case(),
            Span::call_site(),
        );
        let painting_variant_name_str = painting_variant_name.clone();

        let key = quote! { Identifier::vanilla_static(#painting_variant_name_str) };
        let asset_id = generate_identifier(&painting_variant.asset_id);
        let width = painting_variant.width;
        let height = painting_variant.height;
        let title = generate_option(&painting_variant.title, generate_text_component);
        let author = generate_option(&painting_variant.author, generate_text_component);

        stream.extend(quote! {
            pub static #painting_variant_ident: PaintingVariant = PaintingVariant::new(
                #key,
                PaintingVariantValue {
                    width: #width,
                    height: #height,
                    asset_id: #asset_id,
                    title: #title,
                    author: #author,
                },
            );
        });

        register_stream.extend(quote! {
            registry.register(&#painting_variant_ident);
        });
    }

    stream.extend(quote! {
        pub fn register_painting_variants(registry: &mut PaintingVariantRegistry) {
            #register_stream
        }
    });

    stream
}
