use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_SYSTEM_CHAT;
use text_components::EncodedComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SYSTEM_CHAT)]
pub struct CSystemChat {
    pub content: EncodedComponent,
    pub overlay: bool,
}

impl CSystemChat {
    /// `content` must already be resolved for its recipients.
    #[must_use]
    pub fn new(content: impl Into<EncodedComponent>, overlay: bool) -> Self {
        Self {
            content: content.into(),
            overlay,
        }
    }
}
