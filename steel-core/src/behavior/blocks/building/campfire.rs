use std::sync::{Arc, Weak};

use steel_macros::block_behavior;
use steel_registry::{
    REGISTRY, TaggedRegistryExt,
    blocks::{BlockRef, block_state_ext::BlockStateExt, properties::BlockStateProperties},
    fluid::{FluidState, FluidStateExt},
    item_stack::ItemStack,
    items::item::BlockHitResult,
    sound_events, vanilla_block_entity_types, vanilla_block_tags, vanilla_blocks,
    vanilla_damage_types, vanilla_fluids,
};
use steel_utils::{
    BlockPos, BlockStateId, Direction,
    types::{InteractionHand, UpdateFlags},
};

use crate::{
    behavior::{BlockBehavior, BlockPlaceContext, BlockStateBehaviorExt, InteractionResult},
    block_entity::{BLOCK_ENTITIES, SharedBlockEntity},
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
        let replacing_water = if context.replace_clicked {
            context
                .world
                .get_block_state(context.clicked_pos)
                .get_fluid_state()
                .is_water()
        } else {
            false
        };

        Some(
            self.block
                .default_state()
                .set_value(&BlockStateProperties::WATERLOGGED, replacing_water)
                .set_value(
                    &BlockStateProperties::SIGNAL_FIRE,
                    context
                        .world
                        .get_block_state(context.clicked_pos.below())
                        .get_block()
                        == vanilla_blocks::HAY_BLOCK,
                )
                .set_value(&BlockStateProperties::LIT, !replacing_water)
                .set_value(&BlockStateProperties::FACING, context.horizontal_direction),
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
                world.get_block_state(pos.below()).get_block() == vanilla_blocks::HAY_BLOCK,
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
        if state.get_value(&BlockStateProperties::LIT) {
            entity.hurt(
                &DamageSource::environment(vanilla_damage_types::CAMPFIRE),
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
        BLOCK_ENTITIES.create(vanilla_block_entity_types::CAMPFIRE, level, pos, state)
    }

    fn use_item_on(
        &self,
        item_stack: &ItemStack,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        hand: InteractionHand,
        hit_result: &BlockHitResult,
    ) -> InteractionResult {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Pass;
        };
        InteractionResult::Pass
    }
}
