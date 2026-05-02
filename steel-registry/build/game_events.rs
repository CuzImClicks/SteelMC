use std::{collections::BTreeMap, fs};

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

pub fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/game_events.json");

    let json_content = fs::read_to_string("build_assets/game_events.json")
        .expect("Failed to read game_events.json");
    let game_events_file: BTreeMap<String, i32> =
        serde_json::from_str(&json_content).expect("Failed to parse game_events.json");

    let mut constants = TokenStream::new();

    let mut registrations = TokenStream::new();

    for (name, notification_radius) in game_events_file {
        let name = name.strip_prefix("minecraft:").unwrap_or(&name);
        let ident = Ident::new(&name.to_shouty_snake_case(), Span::call_site());
        let key = Literal::string(name);

        constants.extend(quote! {
            pub static #ident: GameEvent = GameEvent {
                key: Identifier::vanilla_static(#key),
                notification_radius: #notification_radius
            };
        });

        registrations.extend(quote! {
            registry.register(&#ident);
        });
    }

    quote! {
        use crate::game_events::{GameEvent, GameEventRegistry};
        use steel_utils::Identifier;

        #constants

        pub fn register_game_events(registry: &mut GameEventRegistry) {
            #registrations
        }
    }
}
