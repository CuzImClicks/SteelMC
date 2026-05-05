//! Block entity implementations.

mod barrel;
mod campfire;
mod sign;

pub use barrel::{BARREL_SLOTS, BarrelBlockEntity};
pub use campfire::{CAMPFIRE_SLOTS, CampfireBlockEntity};
pub use sign::{SIGN_LINES, SignBlockEntity, SignText};
