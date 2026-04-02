use std::{
    any::Any,
    ops::Sub,
    sync::{Arc, Weak},
};

use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::{
    ToNbtTag,
    owned::{NbtCompound, NbtList, NbtTag},
};
use steel_registry::{
    REGISTRY,
    block_entity_type::BlockEntityTypeRef,
    blocks::{block_state_ext::BlockStateExt, properties::BlockStateProperties},
    item_stack::ItemStack,
    vanilla_block_entity_types,
};
use steel_utils::{BlockPos, BlockStateId};

use crate::{block_entity::BlockEntity, inventory::container::Container, world::World};

/// Number of slots in a campfire.
pub const CAMPFIRE_SLOTS: usize = 4;

/// Campfire Block Entity
pub struct CampfireBlockEntity {
    world: Weak<World>,
    pos: BlockPos,
    state: BlockStateId,
    removed: bool,
    items: Vec<ItemStack>,
    cooking_times: Vec<i32>,
    cooking_progress: Vec<i32>,
}

impl CampfireBlockEntity {
    /// Creates a new campfire block entity.
    #[must_use]
    pub fn new(world: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        Self {
            world,
            pos,
            state,
            removed: false,
            items: vec![ItemStack::empty(); CAMPFIRE_SLOTS],
            cooking_times: vec![0; CAMPFIRE_SLOTS],
            cooking_progress: vec![0; CAMPFIRE_SLOTS],
        }
    }

    /// Places food in an available slot in the campfire
    pub fn place_food(&mut self, input: &mut ItemStack, has_infinite_materials: bool) -> bool {
        log::info!("CampfireBlockEntity::place_food");
        for ((item, cooking_time), cooking_progress) in self
            .items
            .iter_mut()
            .zip(self.cooking_times.iter_mut())
            .zip(self.cooking_progress.iter_mut())
        {
            if !item.is_empty() {
                tracing::info!("{item:?} {cooking_time} {cooking_progress}");
                continue;
            }

            let Some(recipe) = REGISTRY
                .recipes
                .iter_campfire()
                .find(|recipe| recipe.matches(input))
            else {
                tracing::error!("wasnt able to find recipe for item - {item:?}");
                return false;
            };

            *cooking_time = recipe.cooking_time;
            *cooking_progress = 0;
            *item = input.copy_with_count(1);
            if !has_infinite_materials {
                input.shrink(1);
            }
            self.set_changed();
            return true;
        }

        false
    }

    fn mark_updated(&self) {
        self.set_changed();

        let Some(world) = self.world.upgrade() else {
            return;
        };
        let mut nbt = NbtCompound::new();
        self.save_additional(&mut nbt);
        world.broadcast_block_entity_update(self.pos, self.get_type(), nbt);
    }
}

impl BlockEntity for CampfireBlockEntity {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_type(&self) -> BlockEntityTypeRef {
        vanilla_block_entity_types::CAMPFIRE
    }

    fn get_block_pos(&self) -> BlockPos {
        self.pos
    }

    fn get_block_state(&self) -> BlockStateId {
        self.state
    }

    fn set_block_state(&mut self, state: BlockStateId) {
        self.state = state;
    }

    fn is_removed(&self) -> bool {
        self.removed
    }

    fn set_removed(&mut self) {
        self.removed = true;
    }

    fn clear_removed(&mut self) {
        self.removed = false;
    }

    fn get_level(&self) -> Option<Arc<World>> {
        self.world.upgrade()
    }

    fn pre_remove_side_effects(&mut self, pos: BlockPos, _state: BlockStateId) {
        if let Some(world) = self.world.upgrade() {
            for item in self.items.drain(..) {
                world.drop_item_stack(pos, item);
            }
        }
    }

    fn load_additional(&mut self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();

        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < CAMPFIRE_SLOTS
                        && let Some(item) = ItemStack::from_borrowed_compound(&compound)
                    {
                        self.items[slot] = item;
                    }
                }
            }
        }

        if let Some(cooking_progress) = nbt_view.int_array("CookingTimes") {
            self.cooking_progress = cooking_progress;
        }

        if let Some(cooking_times) = nbt_view.int_array("CookingTotalTimes") {
            self.cooking_times = cooking_times;
        }
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        let mut items: Vec<NbtCompound> = Vec::new();
        for (slot, item) in self.items.iter().enumerate() {
            if !item.is_empty()
                && let NbtTag::Compound(mut item_nbt) = item.clone().to_nbt_tag()
            {
                item_nbt.insert("Slot", slot as i8);
                items.push(item_nbt);
            }
        }
        nbt.insert("Items", NbtList::Compound(items));
        nbt.insert("CookingTimes", NbtTag::IntArray(self.cooking_times.clone()));
        nbt.insert(
            "CookingTotalTimes",
            NbtTag::IntArray(self.cooking_progress.clone()),
        );
    }

    fn tick(&mut self, world: &Arc<World>) {
        let mut changed = false;
        if self.state.get_value(&BlockStateProperties::LIT) {
            for ((item, cooking_time), cooking_progress) in self
                .items
                .iter_mut()
                .zip(self.cooking_times.iter_mut())
                .zip(self.cooking_progress.iter_mut())
            {
                if item.is_empty() {
                    continue;
                }
                changed = true;
                *cooking_progress += 1;
                if cooking_progress < cooking_time {
                    continue;
                }

                let Some(recipe) = REGISTRY
                    .recipes
                    .iter_campfire()
                    .find(|recipe| recipe.matches(item))
                else {
                    continue;
                };
                world.drop_item_stack(self.pos, recipe.assemble());
                *item = ItemStack::empty();
                *cooking_time = 0;
                *cooking_progress = 0;
            }
        } else {
            for (cooking_time, cooking_progress) in self
                .cooking_times
                .iter_mut()
                .zip(self.cooking_progress.iter_mut())
            {
                if *cooking_progress > 0 {
                    changed = true;
                    *cooking_progress = cooking_progress.sub(2).clamp(0, *cooking_time);
                }
            }
        }

        if changed {
            self.mark_updated();
        }
    }

    fn is_ticking(&self) -> bool {
        true
    }

    fn as_container(&self) -> Option<&(dyn Container + 'static)> {
        Some(self)
    }

    fn as_container_mut(&mut self) -> Option<&mut (dyn Container + 'static)> {
        Some(self)
    }

    fn get_update_tag(&self) -> Option<NbtCompound> {
        let mut nbt = NbtCompound::new();
        self.save_additional(&mut nbt);
        Some(nbt)
    }
}

impl Container for CampfireBlockEntity {
    fn get_container_size(&self) -> usize {
        CAMPFIRE_SLOTS
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        &self.items[slot]
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        &mut self.items[slot]
    }

    fn set_item(&mut self, slot: usize, stack: ItemStack) {
        if slot < CAMPFIRE_SLOTS {
            self.items[slot] = stack;
            self.set_changed();
        }
    }

    fn get_max_stack_size(&self) -> i32 {
        1
    }

    fn set_changed(&mut self) {
        BlockEntity::set_changed(self);
    }
}
