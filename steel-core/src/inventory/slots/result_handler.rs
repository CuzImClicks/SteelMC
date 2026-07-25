use std::sync::Arc;

use enum_dispatch::enum_dispatch;
use steel_registry::item_stack::ItemStack;

use crate::{
    inventory::lock::{ContainerLockGuard, ContainerRef},
    player::Player,
};

/// A trait for recipe handlers that update slots in containers according to recipes
#[enum_dispatch]
pub trait ResultHandler: Send + Sync {
    /// The container the result is written to and read from.
    ///
    /// [`MenuBuilder::result_slot`](crate::inventory::menu::MenuBuilder::result_slot)
    /// derives the slot's container from this, so the handler's writes and the
    /// slot's reads can never target different containers.
    fn result_container(&self) -> ContainerRef;

    /// Recalculate the result based on current inputs.
    fn update_result(&self, guard: &mut ContainerLockGuard);

    /// Consume inputs when the result is taken. Return overflow remainders.
    fn on_result_taken(&self, guard: &mut ContainerLockGuard, player: &Player)
    -> Option<ItemStack>;

    /// Whether the stored result still matches the current inputs.
    fn is_result_valid(&self, guard: &ContainerLockGuard, player: &Player) -> bool;
}

impl<T: ResultHandler + ?Sized> ResultHandler for Arc<T> {
    fn result_container(&self) -> ContainerRef {
        (**self).result_container()
    }

    fn update_result(&self, guard: &mut ContainerLockGuard) {
        (**self).update_result(guard);
    }

    fn on_result_taken(
        &self,
        guard: &mut ContainerLockGuard,
        player: &Player,
    ) -> Option<ItemStack> {
        (**self).on_result_taken(guard, player)
    }

    fn is_result_valid(&self, guard: &ContainerLockGuard, player: &Player) -> bool {
        (**self).is_result_valid(guard, player)
    }
}
