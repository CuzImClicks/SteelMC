//! Flint and steel item behavior with portal ignition.

use crate::behavior::blocks::{CampfireBlock, CandleBlock, CandleCakeBlock, FireBlock};
use crate::behavior::context::{InteractionResult, UseOnContext};
use crate::behavior::item::ItemBehavior;
use steel_macros::item_behavior;
use steel_registry::blocks::block_state_ext::BlockStateExt;
use steel_registry::blocks::properties::BlockStateProperties;
use steel_registry::sound_events;
use steel_registry::vanilla_blocks::FIRE;
use steel_utils::Direction;
use steel_utils::types::UpdateFlags;

/// Behavior for flint and steel items.
#[item_behavior]
pub struct FlintAndSteelItem;

impl ItemBehavior for FlintAndSteelItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        let click_pos = context.hit_result.block_pos;

        let clicked_state = context.world.get_block_state(click_pos);

        if CampfireBlock::can_light(clicked_state)
            || CandleBlock::can_light(clicked_state)
            || CandleCakeBlock::can_light(clicked_state)
        {
            context.world.play_block_sound(
                sound_events::ITEM_FLINTANDSTEEL_USE,
                click_pos,
                1.0,
                rand::random::<f32>() * 0.4 + 0.8,
                Some(context.player.id),
            );
            context.world.set_block(
                click_pos,
                clicked_state.set_value(&BlockStateProperties::LIT, true),
                UpdateFlags::UPDATE_ALL_IMMEDIATE,
            );
            context
                .inv
                .item()
                .hurt_and_break(1, context.player.has_infinite_materials());
            return InteractionResult::Success;
        }

        let fire_pos = click_pos.relative(context.hit_result.direction);
        let (yaw, _) = context.player.rotation.load();
        let forward_dir = Direction::from_yaw(yaw);

        if !FireBlock::can_be_placed_at(context.world, fire_pos, forward_dir) {
            return InteractionResult::Fail;
        }

        context.world.play_block_sound(
            sound_events::ITEM_FLINTANDSTEEL_USE,
            fire_pos,
            1.0,
            rand::random::<f32>() * 0.4 + 0.8,
            Some(context.player.id),
        );

        // TODO: use BaseFireBlock.getState() equivalent to select soul fire vs regular fire
        context
            .world
            .set_block(fire_pos, FIRE.default_state(), UpdateFlags::UPDATE_ALL);

        let has_infinite_materials = context.player.has_infinite_materials();
        context.inv.item().hurt_and_break(1, has_infinite_materials);

        InteractionResult::Success
    }
}
