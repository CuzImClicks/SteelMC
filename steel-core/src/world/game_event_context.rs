//! The context for game events
use steel_utils::BlockStateId;

use crate::entity::Entity;

/// The context for a game event
#[expect(dead_code, reason = "stub")]
#[derive(Clone, Default)]
pub struct GameEventContext<'a> {
    /// The entity that caused the game event
    source_entity: Option<&'a dyn Entity>,
    /// The block state involved in the game event
    affected_state: Option<BlockStateId>,
}

impl<'a> GameEventContext<'a> {
    /// Creates a new `GameEventContext`
    #[must_use]
    pub fn new(
        source_entity: Option<&'a dyn Entity>,
        affected_state: Option<BlockStateId>,
    ) -> Self {
        Self {
            source_entity,
            affected_state,
        }
    }
}
