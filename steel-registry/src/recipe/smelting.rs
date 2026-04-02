use steel_utils::Identifier;

use crate::{
    item_stack::ItemStack,
    recipe::{CraftingCategory, Ingredient, RecipeResult},
};

#[derive(Debug, Clone)]
pub struct SmeltingRecipe {
    pub ident: Identifier,
    pub category: CraftingCategory,
    pub result: RecipeResult,
    pub ingredient: Ingredient,
    pub cooking_time: i32,
    pub experience: f32,
}

impl SmeltingRecipe {
    pub fn matches(&self, input: &ItemStack) -> bool {
        self.ingredient.test(input)
    }

    pub fn assemble(&self) -> ItemStack {
        self.result.to_item_stack()
    }
}
