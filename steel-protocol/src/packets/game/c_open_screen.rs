use std::io::{Result, Write};

use steel_macros::ClientPacket;
use steel_registry::{RegistryEntry, menu_type::MenuTypeRef, packets::play::C_OPEN_SCREEN};
use steel_utils::{codec::VarInt, serial::WriteTo};
use text_components::EncodedComponent;

#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_OPEN_SCREEN)]
pub struct COpenScreen {
    pub container_id: i32,
    pub menu_type: MenuTypeRef,
    pub title: EncodedComponent,
}

impl COpenScreen {
    #[must_use]
    pub fn new(
        container_id: i32,
        menu_type: MenuTypeRef,
        title: impl Into<EncodedComponent>,
    ) -> Self {
        Self {
            container_id,
            menu_type,
            title: title.into(),
        }
    }
}

impl WriteTo for COpenScreen {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        VarInt(self.container_id).write(writer)?;
        VarInt(self.menu_type.id() as i32).write(writer)?;
        self.title.write(writer)?;
        Ok(())
    }
}
