use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_PLAYER_COMBAT_KILL;
use steel_utils::serial::RawComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_PLAYER_COMBAT_KILL)]
pub struct CPlayerCombatKill {
    /// Entity ID of the player that died (should match the client's entity ID).
    #[write(as = VarInt)]
    pub player_id: i32,
    /// The death message.
    pub message: RawComponent,
}

impl CPlayerCombatKill {
    pub fn new(player_id: i32, message: impl Into<RawComponent>) -> Self {
        CPlayerCombatKill {
            player_id,
            message: message.into(),
        }
    }
}
