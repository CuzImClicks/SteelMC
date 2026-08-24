use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_PLAYER_COMBAT_KILL;
use text_components::EncodedComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_PLAYER_COMBAT_KILL)]
pub struct CPlayerCombatKill {
    /// Entity ID of the player that died (should match the client's entity ID).
    #[write(as = VarInt)]
    pub player_id: i32,
    /// The death message.
    pub message: EncodedComponent,
}

impl CPlayerCombatKill {
    pub fn new(player_id: i32, message: impl Into<EncodedComponent>) -> Self {
        CPlayerCombatKill {
            player_id,
            message: message.into(),
        }
    }
}
