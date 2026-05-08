use steel_utils::Identifier;

use crate::{
    item_stack::ItemStack,
    recipe::{CraftingInput, Ingredient, RecipeResult},
};

#[derive(Debug, Clone)]
pub struct StonecuttingRecipe {
    pub ident: Identifier,
    pub result: RecipeResult,
    pub ingredient: Ingredient,
}

impl StonecuttingRecipe {
    pub fn matches(&self, input: &CraftingInput) -> bool {
        self.ingredient.test(input.get(0, 0))
    }

    pub fn assemble(&self) -> ItemStack {
        self.result.to_item_stack()
    }
}
