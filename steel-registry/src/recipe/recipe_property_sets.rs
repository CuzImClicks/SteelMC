use std::sync::OnceLock;

use rustc_hash::FxHashMap;
use steel_utils::Identifier;

use crate::items::ItemRef;

pub struct RecipePropertySet {
    pub key: Identifier,
    pub items: OnceLock<Vec<ItemRef>>,
}

pub type RecipePropertySetRef = &'static RecipePropertySet;

pub struct RecipePropertySetRegistry {
    pending_sets: FxHashMap<Identifier, Vec<ItemRef>>,

    recipe_property_set_by_id: Vec<RecipePropertySetRef>,
    recipe_property_set_by_key: FxHashMap<Identifier, usize>,
    allows_registering: bool,
}

impl RecipePropertySetRegistry {
    pub fn new() -> Self {
        Self {
            pending_sets: FxHashMap::default(),
            recipe_property_set_by_id: Vec::new(),
            recipe_property_set_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }

    pub fn register_with_items(&mut self, set: &'static RecipePropertySet, items: Vec<ItemRef>) {
        self.pending_sets.insert(set.key.clone(), items);
    }

    pub fn add_items(&mut self, key: &Identifier, items: impl IntoIterator<Item = ItemRef>) {
        if !self.allows_registering {
            panic!(
                "Registering new items to RecipePropertySets isn't allowed after `freeze()` has been called on the registry."
            )
        }

        if let Some(entry) = self.pending_sets.get_mut(key) {
            entry.extend(items);
        } else {
            self.pending_sets
                .entry(key.to_owned())
                .or_default()
                .extend(items);
        }
    }
}

impl crate::RegistryExt for RecipePropertySetRegistry {
    type Entry = RecipePropertySet;

    fn freeze(&mut self) {
        for (key, value) in self.pending_sets.drain() {
            if let Some(set) = self
                .recipe_property_set_by_key
                .get(&key)
                .and_then(|&id| self.recipe_property_set_by_id.get(id))
            {
                set.items
                    .set(value)
                    .unwrap_or_else(|_| panic!("failed to put items into `RecipePropertySet`"));
            }
        }
        self.allows_registering = false;
    }

    fn by_id(&self, id: usize) -> Option<&'static RecipePropertySet> {
        self.recipe_property_set_by_id.get(id).copied()
    }

    fn by_key(&self, key: &Identifier) -> Option<&'static RecipePropertySet> {
        self.recipe_property_set_by_key
            .get(key)
            .and_then(|&id| self.recipe_property_set_by_id.get(id).copied())
    }

    fn id_from_key(&self, key: &Identifier) -> Option<usize> {
        self.recipe_property_set_by_key.get(key).copied()
    }

    fn len(&self) -> usize {
        self.recipe_property_set_by_id.len()
    }
    fn is_empty(&self) -> bool {
        self.recipe_property_set_by_id.is_empty()
    }
}

crate::impl_standard_methods!(
    RecipePropertySetRegistry,
    RecipePropertySetRef,
    recipe_property_set_by_id,
    recipe_property_set_by_key,
    allows_registering
);

crate::impl_registry_entry!(RecipePropertySet, recipe_property_sets);
