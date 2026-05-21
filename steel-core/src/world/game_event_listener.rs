//! Game event listener registration and dispatch.

use std::sync::Arc;

use glam::DVec3;
use rustc_hash::FxHashMap;
use steel_registry::game_events::GameEventRef;
use steel_utils::locks::SyncMutex;
use steel_utils::{BlockPos, SectionPos};

use crate::world::World;
use crate::world::game_event_context::GameEventContext;

/// Controls when a listener receives an event during dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventDeliveryMode {
    /// Handle the event immediately while scanning listeners.
    Unspecified,
    /// Queue the event and handle it after sorting by source distance.
    ByDistance,
}

/// A receiver for vanilla game events.
pub trait GameEventListener: Send + Sync {
    /// Returns the current world position of this listener.
    fn listener_pos(&self) -> Option<DVec3>;

    /// Returns the maximum block distance this listener can hear.
    fn listener_radius(&self) -> i32;

    /// Returns how this listener should be ordered during dispatch.
    fn delivery_mode(&self) -> GameEventDeliveryMode {
        GameEventDeliveryMode::Unspecified
    }

    /// Handles a game event from `source_pos`.
    fn handle_game_event(
        &self,
        world: &Arc<World>,
        event: GameEventRef,
        context: &GameEventContext<'_>,
        source_pos: DVec3,
    ) -> bool;
}

/// Shared game event listener handle.
pub type SharedGameEventListener = Arc<dyn GameEventListener>;

struct QueuedListener {
    listener: SharedGameEventListener,
    distance_sq: f64,
}

/// Section-indexed game event listener storage.
#[derive(Default)]
pub struct GameEventListenerStorage {
    listeners_by_section: SyncMutex<FxHashMap<SectionPos, Vec<SharedGameEventListener>>>,
}

impl GameEventListenerStorage {
    /// Creates empty game event listener storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `listener` in `section_pos`.
    pub fn register(&self, section_pos: SectionPos, listener: SharedGameEventListener) {
        let mut listeners_by_section = self.listeners_by_section.lock();
        let listeners = listeners_by_section.entry(section_pos).or_default();
        if listeners
            .iter()
            .any(|existing| Arc::ptr_eq(existing, &listener))
        {
            return;
        }
        listeners.push(listener);
    }

    /// Unregisters `listener` from `section_pos`.
    pub fn unregister(&self, section_pos: SectionPos, listener: &SharedGameEventListener) -> bool {
        let mut listeners_by_section = self.listeners_by_section.lock();
        let Some(listeners) = listeners_by_section.get_mut(&section_pos) else {
            return false;
        };

        let old_len = listeners.len();
        listeners.retain(|existing| !Arc::ptr_eq(existing, listener));
        let removed = listeners.len() != old_len;
        let is_empty = listeners.is_empty();

        if is_empty {
            listeners_by_section.remove(&section_pos);
        }

        removed
    }

    /// Dispatches `event` to listeners in range and returns the handled count.
    pub fn dispatch(
        &self,
        world: &Arc<World>,
        event: GameEventRef,
        source_pos: DVec3,
        context: &GameEventContext<'_>,
    ) -> usize {
        let mut by_distance = Vec::new();
        let mut handled = 0;

        for queued in self.collect_in_range(source_pos, event.notification_radius) {
            if queued.listener.delivery_mode() == GameEventDeliveryMode::ByDistance {
                by_distance.push(queued);
            } else if queued
                .listener
                .handle_game_event(world, event, context, source_pos)
            {
                handled += 1;
            }
        }

        by_distance.sort_by(|left, right| left.distance_sq.total_cmp(&right.distance_sq));
        for queued in by_distance {
            if queued
                .listener
                .handle_game_event(world, event, context, source_pos)
            {
                handled += 1;
            }
        }

        handled
    }

    fn collect_in_range(&self, source_pos: DVec3, notification_radius: i32) -> Vec<QueuedListener> {
        let notification_radius = notification_radius.max(0);
        let source_block_pos = BlockPos::from(source_pos);
        let section_min_x =
            SectionPos::block_to_section_coord(source_block_pos.x() - notification_radius);
        let section_min_y =
            SectionPos::block_to_section_coord(source_block_pos.y() - notification_radius);
        let section_min_z =
            SectionPos::block_to_section_coord(source_block_pos.z() - notification_radius);
        let section_max_x =
            SectionPos::block_to_section_coord(source_block_pos.x() + notification_radius);
        let section_max_y =
            SectionPos::block_to_section_coord(source_block_pos.y() + notification_radius);
        let section_max_z =
            SectionPos::block_to_section_coord(source_block_pos.z() + notification_radius);

        let listeners = {
            let listeners_by_section = self.listeners_by_section.lock();
            let mut listeners = Vec::new();
            for section_x in section_min_x..=section_max_x {
                for section_z in section_min_z..=section_max_z {
                    for section_y in section_min_y..=section_max_y {
                        let section_pos = SectionPos::new(section_x, section_y, section_z);
                        if let Some(section_listeners) = listeners_by_section.get(&section_pos) {
                            listeners.extend(section_listeners.iter().map(Arc::clone));
                        }
                    }
                }
            }
            listeners
        };

        let mut in_range = Vec::new();
        for listener in listeners {
            let Some(listener_pos) = listener.listener_pos() else {
                continue;
            };
            let block_distance_sq =
                block_distance_sq(source_block_pos, BlockPos::from(listener_pos));
            let listener_radius = listener.listener_radius().max(0);
            let listener_radius_sq = i64::from(listener_radius) * i64::from(listener_radius);
            if block_distance_sq <= listener_radius_sq {
                in_range.push(QueuedListener {
                    listener,
                    distance_sq: exact_distance_sq(source_pos, listener_pos),
                });
            }
        }

        in_range
    }
}

fn block_distance_sq(left: BlockPos, right: BlockPos) -> i64 {
    let dx = i64::from(left.x()) - i64::from(right.x());
    let dy = i64::from(left.y()) - i64::from(right.y());
    let dz = i64::from(left.z()) - i64::from(right.z());
    dx * dx + dy * dy + dz * dz
}

fn exact_distance_sq(left: DVec3, right: DVec3) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use glam::DVec3;
    use steel_registry::game_events::GameEventRef;
    use steel_utils::{BlockPos, SectionPos};

    use crate::world::World;
    use crate::world::game_event_context::GameEventContext;
    use crate::world::game_event_listener::{
        GameEventListener, GameEventListenerStorage, SharedGameEventListener,
    };

    struct FixedListener {
        pos: DVec3,
        radius: i32,
    }

    impl GameEventListener for FixedListener {
        fn listener_pos(&self) -> Option<DVec3> {
            Some(self.pos)
        }

        fn listener_radius(&self) -> i32 {
            self.radius
        }

        fn handle_game_event(
            &self,
            _world: &Arc<World>,
            _event: GameEventRef,
            _context: &GameEventContext<'_>,
            _source_pos: DVec3,
        ) -> bool {
            false
        }
    }

    #[test]
    fn collect_in_range_filters_by_listener_radius() {
        let storage = GameEventListenerStorage::new();
        let near: SharedGameEventListener = Arc::new(FixedListener {
            pos: DVec3::new(2.0, 64.0, 0.0),
            radius: 16,
        });
        let far: SharedGameEventListener = Arc::new(FixedListener {
            pos: DVec3::new(32.0, 64.0, 0.0),
            radius: 16,
        });

        storage.register(
            SectionPos::from_block_pos(BlockPos::new(2, 64, 0)),
            Arc::clone(&near),
        );
        storage.register(
            SectionPos::from_block_pos(BlockPos::new(32, 64, 0)),
            Arc::clone(&far),
        );

        let matches = storage.collect_in_range(DVec3::new(0.5, 64.5, 0.5), 64);

        assert_eq!(matches.len(), 1);
        assert!(Arc::ptr_eq(&matches[0].listener, &near));
    }

    #[test]
    fn unregister_removes_empty_section_bucket() {
        let storage = GameEventListenerStorage::new();
        let listener: SharedGameEventListener = Arc::new(FixedListener {
            pos: DVec3::new(0.0, 64.0, 0.0),
            radius: 16,
        });
        let section_pos = SectionPos::new(0, 4, 0);

        storage.register(section_pos, Arc::clone(&listener));

        assert!(storage.unregister(section_pos, &listener));
        assert!(
            storage
                .collect_in_range(DVec3::new(0.5, 64.5, 0.5), 16)
                .is_empty()
        );
    }

    #[test]
    fn collect_in_range_records_exact_distance_for_delivery_sorting() {
        let storage = GameEventListenerStorage::new();
        let listener: SharedGameEventListener = Arc::new(FixedListener {
            pos: DVec3::new(0.1, 64.5, 0.5),
            radius: 16,
        });

        storage.register(
            SectionPos::from_block_pos(BlockPos::new(0, 64, 0)),
            Arc::clone(&listener),
        );

        let matches = storage.collect_in_range(DVec3::new(0.9, 64.5, 0.5), 16);

        assert_eq!(matches.len(), 1);
        assert!((matches[0].distance_sq - 0.64).abs() < f64::EPSILON);
    }
}
