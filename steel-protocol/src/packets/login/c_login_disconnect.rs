use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::login::C_LOGIN_DISCONNECT;
use text_components::TextComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = C_LOGIN_DISCONNECT)]
pub struct CLoginDisconnect {
    #[write(as = Json)]
    pub reason: TextComponent,
}

impl CLoginDisconnect {
    #[must_use]
    pub fn new(reason: impl Into<TextComponent>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}
