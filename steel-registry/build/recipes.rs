//! Build script for generating vanilla recipe definitions.
//!
//! This module generates recipe definitions using a hybrid approach:
//! - `LazyLock` for the RECIPES struct (required because ITEMS uses LazyLock)
//! - `Box::leak` to create `&'static [Ingredient]` slices at runtime
//! - `#[inline(never)]` creator functions to prevent stack overflow
//!
//! The `Box::leak` pattern is intentional: vanilla recipes live for the entire
//! program lifetime, so leaking the memory is correct. This gives us zero-cost
//! access to recipe data after initialization.

use std::{fs, path::Path};

use heck::ToSnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct RecipeJson {
    #[serde(rename = "type")]
    recipe_type: String,
    #[serde(default)]
    category: Option<String>,

    // Shaped recipe fields
    #[serde(default)]
    key: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pattern: Option<Vec<String>>,

    // Shapeless recipe fields
    #[serde(default)]
    ingredients: Option<Vec<Value>>,

    // Smelting recipe fields
    #[serde(default)]
    ingredient: Option<Value>,

    #[serde(default, rename = "cookingtime")]
    cooking_time: Option<i32>,
    #[serde(default)]
    experience: Option<f32>,

    // Common fields
    #[serde(default)]
    result: Option<RecipeResult>,
    #[serde(default)]
    show_notification: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct RecipeResult {
    id: String,
    #[serde(default = "default_count")]
    count: i32,
}

fn default_count() -> i32 {
    1
}

/// Represents a parsed ingredient from JSON.
#[derive(Clone, Debug)]
enum ParsedIngredient {
    Empty,
    Item(String),        // item identifier
    Tag(String),         // tag identifier
    Choice(Vec<String>), // list of item identifiers
}

/// Parses an ingredient from a JSON value.
fn parse_ingredient(value: &Value) -> ParsedIngredient {
    match value {
        Value::String(s) => {
            if let Some(tag) = s.strip_prefix('#') {
                let tag_id = tag.strip_prefix("minecraft:").unwrap_or(tag);
                ParsedIngredient::Tag(tag_id.to_string())
            } else {
                let item_id = s.strip_prefix("minecraft:").unwrap_or(s);
                ParsedIngredient::Item(item_id.to_string())
            }
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| {
                    let item_id = s.strip_prefix("minecraft:").unwrap_or(s);
                    item_id.to_string()
                })
                .collect();
            ParsedIngredient::Choice(items)
        }
        Value::Object(obj) => {
            if let Some(item) = obj.get("item").and_then(|v| v.as_str()) {
                let item_id = item.strip_prefix("minecraft:").unwrap_or(item);
                ParsedIngredient::Item(item_id.to_string())
            } else if let Some(tag) = obj.get("tag").and_then(|v| v.as_str()) {
                let tag_id = tag.strip_prefix("minecraft:").unwrap_or(tag);
                ParsedIngredient::Tag(tag_id.to_string())
            } else {
                ParsedIngredient::Empty
            }
        }
        _ => ParsedIngredient::Empty,
    }
}

struct ShapedRecipeData {
    name: String,
    ident: Ident,
    category: TokenStream,
    width: usize,
    height: usize,
    pattern_data: Vec<ParsedIngredient>,
    result_item_ident: Ident,
    result_count: i32,
    show_notification: bool,
    symmetrical: bool,
}

struct ShapelessRecipeData {
    name: String,
    ident: Ident,
    category: TokenStream,
    ingredient_data: Vec<ParsedIngredient>,
    result_item_ident: Ident,
    result_count: i32,
}

/// Parses a shaped recipe from JSON.
fn parse_shaped_recipe(recipe_name: &str, recipe: &RecipeJson) -> Option<ShapedRecipeData> {
    let pattern = recipe.pattern.as_ref()?;
    let key = recipe.key.as_ref()?;
    let result = recipe.result.as_ref()?;

    // Calculate width and height from pattern
    let height = pattern.len();
    let width = pattern
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);

    // Build ingredient map from key
    let mut ingredient_map: FxHashMap<char, ParsedIngredient> = FxHashMap::default();
    ingredient_map.insert(' ', ParsedIngredient::Empty);

    for (key_char, value) in key {
        if let Some(c) = key_char.chars().next() {
            ingredient_map.insert(c, parse_ingredient(value));
        }
    }

    // Build pattern vector and character grid for symmetry check
    let mut pattern_data = Vec::new();
    let mut char_grid: Vec<char> = Vec::new();
    for row in pattern {
        // Pad row to width
        let padded: String = format!("{:width$}", row, width = width);
        for c in padded.chars() {
            char_grid.push(c);
            let ingredient = ingredient_map
                .get(&c)
                .cloned()
                .unwrap_or(ParsedIngredient::Empty);
            pattern_data.push(ingredient);
        }
    }

    // Check horizontal symmetry using the character grid
    let symmetrical = is_pattern_symmetrical(width, height, &char_grid);

    // Result item
    let result_item_id = result.id.strip_prefix("minecraft:").unwrap_or(&result.id);
    let result_item_ident = Ident::new(result_item_id, Span::call_site());

    // Category
    let category_str = recipe.category.as_deref().unwrap_or("misc");
    let category = category_to_tokens(category_str);

    let snake_name = recipe_name.to_snake_case();

    Some(ShapedRecipeData {
        name: recipe_name.to_string(),
        ident: Ident::new(&snake_name, Span::call_site()),
        category,
        width,
        height,
        pattern_data,
        result_item_ident,
        result_count: result.count,
        show_notification: recipe.show_notification.unwrap_or(true),
        symmetrical,
    })
}

/// Checks if a pattern is horizontally symmetric.
fn is_pattern_symmetrical(width: usize, height: usize, chars: &[char]) -> bool {
    if width == 0 {
        return true;
    }
    for y in 0..height {
        for x in 0..width / 2 {
            let left = chars[y * width + x];
            let right = chars[y * width + (width - 1 - x)];
            if left != right {
                return false;
            }
        }
    }
    true
}

/// Parses a shapeless recipe from JSON.
fn parse_shapeless_recipe(recipe_name: &str, recipe: &RecipeJson) -> Option<ShapelessRecipeData> {
    let ingredients = recipe.ingredients.as_ref()?;
    let result = recipe.result.as_ref()?;

    // Build ingredients vector
    let ingredient_data: Vec<ParsedIngredient> = ingredients.iter().map(parse_ingredient).collect();

    // Result item
    let result_item_id = result.id.strip_prefix("minecraft:").unwrap_or(&result.id);
    let result_item_ident = Ident::new(result_item_id, Span::call_site());

    // Category
    let category_str = recipe.category.as_deref().unwrap_or("misc");
    let category = category_to_tokens(category_str);

    let snake_name = recipe_name.to_snake_case();

    Some(ShapelessRecipeData {
        name: recipe_name.to_string(),
        ident: Ident::new(&snake_name, Span::call_site()),
        category,
        ingredient_data,
        result_item_ident,
        result_count: result.count,
    })
}

/// Generates a TokenStream for an ingredient.
/// For Choice ingredients, uses Box::leak to create a static slice.
fn generate_ingredient_tokens(ingredient: &ParsedIngredient) -> TokenStream {
    match ingredient {
        ParsedIngredient::Empty => quote! { Ingredient::Empty },
        ParsedIngredient::Item(item_id) => {
            let item_ident = Ident::new(item_id, Span::call_site());
            quote! { Ingredient::Item(&ITEMS.#item_ident) }
        }
        ParsedIngredient::Tag(tag_id) => {
            quote! { Ingredient::Tag(Identifier::vanilla_static(#tag_id)) }
        }
        ParsedIngredient::Choice(items) => {
            let item_refs: Vec<TokenStream> = items
                .iter()
                .map(|item_id| {
                    let item_ident = Ident::new(item_id, Span::call_site());
                    quote! { &ITEMS.#item_ident }
                })
                .collect();
            // Use Box::leak to create a static slice for Choice items
            quote! {
                Ingredient::Choice(Box::leak(Box::new([#(#item_refs),*])))
            }
        }
    }
}

fn category_to_tokens(category: &str) -> TokenStream {
    match category {
        "building" => quote! { CraftingCategory::Building },
        "redstone" => quote! { CraftingCategory::Redstone },
        "equipment" => quote! { CraftingCategory::Equipment },
        "food" => quote! { CraftingCategory::Food },
        _ => quote! { CraftingCategory::Misc },
    }
}

struct SmeltingRecipeData {
    name: String,
    ident: Ident,
    category: TokenStream,
    cooking_time: i32,
    experience: f32,
    ingredient: ParsedIngredient,
    result: Ident,
}

fn parse_smelting_recipe(recipe_name: &str, data: &RecipeJson) -> Option<SmeltingRecipeData> {
    let category = data.category.as_ref()?;

    let category = category_to_tokens(category);

    let cooking_time = data.cooking_time?;
    let experience = data.experience?;
    let ingredient = data.ingredient.as_ref()?;
    let result = data.result.as_ref()?;

    let result_item_id = result.id.strip_prefix("minecraft:").unwrap_or(&result.id);
    let result_item_ident = Ident::new(result_item_id, Span::call_site());

    let ingredient = parse_ingredient(ingredient);

    Some(SmeltingRecipeData {
        name: recipe_name.to_string(),
        ident: Ident::new(&recipe_name.to_snake_case(), Span::call_site()),
        category,
        cooking_time,
        experience,
        ingredient,
        result: result_item_ident,
    })
}

struct RecipeCodegen {
    creator_fns: Vec<TokenStream>,
    fields: Vec<TokenStream>,
    field_inits: Vec<TokenStream>,
    registers: Vec<TokenStream>,
}

fn generate_codegen(
    prefix: &str,
    recipe_type: &str,
    recipes: &[(Ident, TokenStream)],
) -> RecipeCodegen {
    let type_ident = Ident::new(recipe_type, Span::call_site());

    let mut cg = RecipeCodegen {
        creator_fns: Vec::new(),
        fields: Vec::new(),
        field_inits: Vec::new(),
        registers: Vec::new(),
    };

    for (ident, body) in recipes {
        let fn_ident = Ident::new(&format!("create_{prefix}_{ident}"), Span::call_site());
        let register_method = Ident::new(&format!("register_{prefix}"), Span::call_site());
        let prefix_ident = Ident::new(prefix, Span::call_site());

        cg.creator_fns.push(quote! {
            #[inline(never)]
            fn #fn_ident() -> #type_ident { #body }
        });
        cg.fields.push(quote! { pub #ident: #type_ident, });
        cg.field_inits.push(quote! { #ident: #fn_ident(), });
        cg.registers.push(quote! {
            registry.#register_method(&RECIPES.#prefix_ident.#ident);
        });
    }

    cg
}

fn generate_shaped_body(r: &ShapedRecipeData) -> TokenStream {
    let name = &r.name;
    let category = &r.category;
    let width = r.width;
    let height = r.height;
    let result_item_ident = &r.result_item_ident;
    let result_count = r.result_count;
    let show_notification = r.show_notification;
    let symmetrical = r.symmetrical;

    let pattern_tokens: Vec<TokenStream> = r
        .pattern_data
        .iter()
        .map(generate_ingredient_tokens)
        .collect();

    quote! {
        let pattern: &'static [Ingredient] = Box::leak(
            vec![#(#pattern_tokens),*].into_boxed_slice()
        );
        ShapedRecipe {
            id: Identifier::vanilla_static(#name),
            category: #category,
            width: #width,
            height: #height,
            pattern,
            result: RecipeResult {
                item: &ITEMS.#result_item_ident,
                count: #result_count,
            },
            show_notification: #show_notification,
            symmetrical: #symmetrical,
        }
    }
}

fn generate_shapeless_body(r: &ShapelessRecipeData) -> TokenStream {
    let name = &r.name;
    let category = &r.category;
    let result_item_ident = &r.result_item_ident;
    let result_count = r.result_count;

    let ingredient_tokens: Vec<TokenStream> = r
        .ingredient_data
        .iter()
        .map(generate_ingredient_tokens)
        .collect();

    quote! {

        // Box::leak creates a &'static [Ingredient] from the Vec.
        // This is intentional: vanilla recipes live forever.
        let ingredients: &'static [Ingredient] = Box::leak(
            vec![#(#ingredient_tokens),*].into_boxed_slice()
        );
        ShapelessRecipe {
            id: Identifier::vanilla_static(#name),
            category: #category,
            ingredients,
            result: RecipeResult {
                item: &ITEMS.#result_item_ident,
                count: #result_count,
            },
        }
    }
}

fn generate_smelting_body(r: &SmeltingRecipeData) -> TokenStream {
    let name = &r.name;
    let category = &r.category;
    let result_item_ident = &r.result;

    let ingredient_tokens: TokenStream = generate_ingredient_tokens(&r.ingredient);

    let cooking_time = r.cooking_time;
    let experience = r.experience;

    quote! {
        SmeltingRecipe {
            ident: Identifier::vanilla_static(#name),
            category: #category,
            ingredient: #ingredient_tokens,
            result: RecipeResult {
                item: &ITEMS.#result_item_ident,
                count: 1,
            },
            cooking_time: #cooking_time,
            experience: #experience,
        }
    }
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/builtin_datapacks/minecraft/recipe/");

    let recipe_dir = "build_assets/builtin_datapacks/minecraft/recipe";

    let mut shaped_recipes: Vec<ShapedRecipeData> = Vec::new();
    let mut shapeless_recipes: Vec<ShapelessRecipeData> = Vec::new();
    let mut smelting_recipes: Vec<SmeltingRecipeData> = Vec::new();
    let mut campfire_recipes: Vec<SmeltingRecipeData> = Vec::new();
    let mut smoking_recipes: Vec<SmeltingRecipeData> = Vec::new();

    // Read all recipe files
    fn read_recipes(
        dir: &Path,
        shaped: &mut Vec<ShapedRecipeData>,
        shapeless: &mut Vec<ShapelessRecipeData>,
        smelting: &mut Vec<SmeltingRecipeData>,
        campfire: &mut Vec<SmeltingRecipeData>,
        smoking: &mut Vec<SmeltingRecipeData>,
    ) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                read_recipes(&path, shaped, shapeless, smelting, campfire, smoking);
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let recipe_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                let content = match fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let recipe: RecipeJson = match serde_json::from_str(&content) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                match recipe.recipe_type.as_str() {
                    "minecraft:crafting_shaped" => {
                        if let Some(r) = parse_shaped_recipe(recipe_name, &recipe) {
                            shaped.push(r);
                        }
                    }
                    "minecraft:crafting_shapeless" => {
                        if let Some(r) = parse_shapeless_recipe(recipe_name, &recipe) {
                            shapeless.push(r);
                        }
                    }
                    "minecraft:smelting" => {
                        if let Some(r) = parse_smelting_recipe(recipe_name, &recipe) {
                            smelting.push(r);
                        }
                    }
                    "minecraft:campfire_cooking" => {
                        if let Some(r) = parse_smelting_recipe(recipe_name, &recipe) {
                            campfire.push(r);
                        }
                    }
                    "minecraft:smoking_cooking" => {
                        if let Some(r) = parse_smelting_recipe(recipe_name, &recipe) {
                            smoking.push(r);
                        }
                    }
                    // Skip other recipe types for now (smelting, stonecutting, smithing, etc.)
                    _ => {}
                }
            }
        }
    }

    read_recipes(
        Path::new(recipe_dir),
        &mut shaped_recipes,
        &mut shapeless_recipes,
        &mut smelting_recipes,
        &mut campfire_recipes,
        &mut smoking_recipes,
    );

    let shaped = generate_codegen(
        "shaped",
        "ShapedRecipe",
        &shaped_recipes
            .iter()
            .map(|recipe| (recipe.ident.clone(), generate_shaped_body(recipe)))
            .collect::<Vec<(Ident, TokenStream)>>(),
    );

    let shapeless = generate_codegen(
        "shapeless",
        "ShapelessRecipe",
        &shapeless_recipes
            .iter()
            .map(|recipe| (recipe.ident.clone(), generate_shapeless_body(recipe)))
            .collect::<Vec<(Ident, TokenStream)>>(),
    );

    let smelting = generate_codegen(
        "smelting",
        "SmeltingRecipe",
        &smelting_recipes
            .iter()
            .map(|recipe| (recipe.ident.clone(), generate_smelting_body(recipe)))
            .collect::<Vec<(Ident, TokenStream)>>(),
    );

    let campfire = generate_codegen(
        "campfire",
        "SmeltingRecipe",
        &campfire_recipes
            .iter()
            .map(|recipe| (recipe.ident.clone(), generate_smelting_body(recipe)))
            .collect::<Vec<(Ident, TokenStream)>>(),
    );

    let smoking = generate_codegen(
        "smoking",
        "SmeltingRecipe",
        &smoking_recipes
            .iter()
            .map(|recipe| (recipe.ident.clone(), generate_smelting_body(recipe)))
            .collect::<Vec<(Ident, TokenStream)>>(),
    );

    let all_creator_fns: Vec<&TokenStream> = shaped
        .creator_fns
        .iter()
        .chain(shapeless.creator_fns.iter())
        .chain(smelting.creator_fns.iter())
        .chain(campfire.creator_fns.iter())
        .chain(smoking.creator_fns.iter())
        .collect();

    let all_registers: Vec<&TokenStream> = shaped
        .registers
        .iter()
        .chain(shapeless.registers.iter())
        .chain(smelting.registers.iter())
        .chain(campfire.registers.iter())
        .chain(smoking.registers.iter())
        .collect();

    let shaped_fields = &shaped.fields;
    let shaped_field_inits = &shaped.field_inits;

    let shapeless_fields = &shapeless.fields;
    let shapeless_field_inits = &shapeless.field_inits;

    let smelting_fields = &smelting.fields;
    let smelting_field_inits = &smelting.field_inits;

    let campfire_fields = &campfire.fields;
    let campfire_field_inits = &campfire.field_inits;

    let smoking_fields = &smoking.fields;
    let smoking_field_inits = &smoking.field_inits;

    quote! {
        use crate::{
            recipe::{
                CraftingCategory, Ingredient, RecipeRegistry, RecipeResult,
                ShapedRecipe, ShapelessRecipe, SmeltingRecipe
            },
            vanilla_items::ITEMS,
        };
        use steel_utils::Identifier;
        use std::sync::LazyLock;

        /// Global vanilla recipes instance.
        ///
        /// Uses `LazyLock` for thread-safe lazy initialization.
        /// Recipe data (patterns/ingredients) uses `Box::leak` to create
        /// `&'static` slices, providing zero-cost access after initialization.
        pub static RECIPES: LazyLock<Recipes> = LazyLock::new(Recipes::init);

        pub struct ShapedRecipes { #(#shaped_fields)* }
        pub struct ShapelessRecipes { #(#shapeless_fields)* }
        pub struct SmeltingRecipes { #(#smelting_fields)* }
        pub struct CampfireRecipes { #(#campfire_fields)* }
        pub struct SmokingRecipes { #(#smoking_fields)* }

        pub struct Recipes {
            pub shaped: ShapedRecipes,
            pub shapeless: ShapelessRecipes,
            pub smelting: SmeltingRecipes,
            pub campfire: CampfireRecipes,
            pub smoking: SmokingRecipes,
        }

        // Individual recipe creator functions.
        //
        // Each function is marked `#[inline(never)]` to ensure it gets its own
        // stack frame. This prevents stack overflow that would occur if all
        // recipes were initialized in a single large struct literal.
        //
        // Each function uses `Box::leak` to convert the ingredient Vec into
        // a `&'static [Ingredient]`. This is intentional and correct:
        // - Vanilla recipes live for the entire program lifetime
        // - The leaked memory is a one-time cost at startup
        // - Access to recipe data after init is zero-cost (just pointer + length)
        #(#all_creator_fns)*

        impl Recipes {
            fn init() -> Self {
                Self {
                    shaped: ShapedRecipes {
                        #(#shaped_field_inits)*
                    },
                    shapeless: ShapelessRecipes {
                        #(#shapeless_field_inits)*
                    },
                    smelting: SmeltingRecipes {
                        #(#smelting_field_inits)*
                    },
                    campfire: CampfireRecipes {
                        #(#campfire_field_inits)*
                    },
                    smoking: SmokingRecipes {
                        #(#smoking_field_inits)*
                    }
                }
            }
        }

        /// Registers all vanilla recipes with the recipe registry.
        pub fn register_recipes(registry: &mut RecipeRegistry) {
            // Force initialization of RECIPES
            let _ = &*RECIPES;
            #(#all_registers)*
        }
    }
}
