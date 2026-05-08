use steel_utils::Identifier;

use crate::{
    item_stack::ItemStack,
    recipe::{CraftingCategory, CraftingInput, Ingredient, RecipeResult},
};

#[derive(Debug, Clone)]
pub struct SmeltingRecipe {
    pub ident: Identifier,
    pub category: CraftingCategory,
    pub result: RecipeResult,
    pub ingredient: Ingredient,
    pub group: Option<&'static str>,
    pub cooking_time: i32,
    pub experience: f32,
}

impl SmeltingRecipe {
    pub fn matches(&self, input: &CraftingInput) -> bool {
        self.ingredient.test(input.get(0, 0))
    }

    pub fn assemble(&self) -> ItemStack {
        self.result.to_item_stack()
    }
}
