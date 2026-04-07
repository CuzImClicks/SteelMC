use std::sync::{Arc, Weak};

use steel_macros::block_behavior;
use steel_registry::{
    REGISTRY, TaggedRegistryExt,
    blocks::{BlockRef, block_state_ext::BlockStateExt, properties::BlockStateProperties},
    fluid::{FluidState, FluidStateExt},
    items::item::BlockHitResult,
    sound_events, vanilla_block_entity_types, vanilla_block_tags, vanilla_blocks,
    vanilla_damage_types, vanilla_fluids, vanilla_recipe_property_sets,
};
use steel_utils::{
    BlockPos, BlockStateId, Direction,
    types::{InteractionHand, UpdateFlags},
};

use crate::{
    behavior::{BlockBehavior, BlockPlaceContext, InteractionResult, InventoryAccess},
    block_entity::{BLOCK_ENTITIES, SharedBlockEntity, entities::CampfireBlockEntity},
    entity::{Entity, damage::DamageSource},
    player::Player,
    world::World,
};

/// Behavior for Campfires
/// - [ ] projectile hit
/// - [ ] getTicker to run cooking ticks
#[block_behavior]
pub struct CampfireBlock {
    block: BlockRef,
    #[json_arg(value)]
    fire_damage: i32,
}

impl CampfireBlock {
    /// Creates a new Campfire Block Behavior
    #[must_use]
    pub const fn new(block: BlockRef, fire_damage: i32) -> Self {
        Self { block, fire_damage }
    }

    /// Whether or not the Campfire can be lit
    #[must_use]
    pub fn can_light(state: BlockStateId) -> bool {
        REGISTRY
            .blocks
            .is_in_tag(state.get_block(), &vanilla_block_tags::CAMPFIRES_TAG)
            && !state
                .try_get_value(&BlockStateProperties::WATERLOGGED)
                .unwrap_or(true)
            && !state
                .try_get_value(&BlockStateProperties::LIT)
                .unwrap_or(true)
    }
}

impl BlockBehavior for CampfireBlock {
    fn get_state_for_placement(
        &self,
        context: &BlockPlaceContext<'_>,
    ) -> Option<steel_utils::BlockStateId> {
        let is_replacing_water = context.is_water_source();

        Some(
            self.block
                .default_state()
                .set_value(&BlockStateProperties::WATERLOGGED, is_replacing_water)
                .set_value(
                    &BlockStateProperties::SIGNAL_FIRE,
                    context
                        .world
                        .get_block_state(context.relative_pos.below())
                        .get_block()
                        == &vanilla_blocks::HAY_BLOCK,
                )
                .set_value(&BlockStateProperties::LIT, !is_replacing_water)
                .set_value(
                    &BlockStateProperties::HORIZONTAL_FACING,
                    context.horizontal_direction,
                ),
        )
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        if state.get_value(&BlockStateProperties::WATERLOGGED) {
            world.schedule_fluid_tick_default(
                pos,
                &vanilla_fluids::WATER,
                vanilla_fluids::WATER.tick_delay as i32,
            );
        }

        if direction == Direction::Down {
            return state.set_value(
                &BlockStateProperties::SIGNAL_FIRE,
                world.get_block_state(pos.below()).get_block() == &vanilla_blocks::HAY_BLOCK,
            );
        }
        state
    }

    fn entity_inside(
        &self,
        state: BlockStateId,
        _world: &Arc<World>,
        _pos: BlockPos,
        entity: &dyn Entity,
    ) {
        if state.get_value(&BlockStateProperties::LIT)
            && let Some(living) = entity.as_living()
        {
            living.hurt(
                &DamageSource::environment(&vanilla_damage_types::CAMPFIRE),
                self.fire_damage as f32,
            );
        }
    }

    fn get_fluid_state(&self, state: BlockStateId) -> FluidState {
        if state.get_value(&BlockStateProperties::WATERLOGGED) {
            FluidState::source(&vanilla_fluids::WATER)
        } else {
            FluidState::EMPTY
        }
    }

    fn place_liquid(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        fluid_state: FluidState,
    ) -> bool {
        if state.get_value(&BlockStateProperties::WATERLOGGED) || !fluid_state.is_water() {
            return false;
        }

        if !fluid_state.is_source() {
            // this is a really weird fix for placing campfires not being destroyed when being placed under a water source
            return true;
        }

        if state.get_value(&BlockStateProperties::LIT) {
            world.play_block_sound(
                sound_events::ENTITY_GENERIC_EXTINGUISH_FIRE,
                pos,
                1.0,
                1.0,
                None,
            );
        }
        world.set_block(
            pos,
            state
                .set_value(&BlockStateProperties::WATERLOGGED, true)
                .set_value(&BlockStateProperties::LIT, false),
            UpdateFlags::UPDATE_ALL,
        );

        world.schedule_fluid_tick_default(
            pos,
            fluid_state.fluid_id,
            fluid_state.fluid_id.tick_delay as i32,
        );

        true
    }

    fn has_block_entity(&self) -> bool {
        true
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> Option<SharedBlockEntity> {
        BLOCK_ENTITIES.create(&vanilla_block_entity_types::CAMPFIRE, level, pos, state)
    }

    fn use_item_on(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hand: InteractionHand,
        _hit_result: &BlockHitResult,
        inv: &mut InventoryAccess,
    ) -> InteractionResult {
        log::info!("CampfireBlock::use_item_on");
        let item_stack = inv.item();
        let Some(block_entity) = world.get_block_entity(pos) else {
            tracing::info!("No Block Entity found");
            return InteractionResult::TryEmptyHandInteraction;
        };

        if !vanilla_recipe_property_sets::CAMPFIRE_INPUT.contains(&item_stack.item()) {
            tracing::info!("Item isnt a valid input for a Campfire");
            return InteractionResult::TryEmptyHandInteraction;
        }

        let mut lock = block_entity.lock();

        let Some(campfire_entity) = lock.as_any_mut().downcast_mut::<CampfireBlockEntity>() else {
            tracing::info!("Wasnt able to convert the BlockEntity into a CampfireBlockEntity");
            return InteractionResult::Fail;
        };

        if campfire_entity.place_food(item_stack, player.has_infinite_materials()) {
            return InteractionResult::Success;
        }

        tracing::info!("Failed to place food in campfire");

        InteractionResult::TryEmptyHandInteraction
    }
}
