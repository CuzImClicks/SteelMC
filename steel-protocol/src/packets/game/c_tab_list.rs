use steel_macros::ClientPacket;
use steel_registry::packets::play::C_TAB_LIST;
use text_components::EncodedComponent;

/// Packet to set the tab list header and footer.
/// This allows servers to display custom text above and below the player list.
#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_TAB_LIST)]
pub struct CTabList {
    /// The header text component (displayed above the player list)
    pub header: EncodedComponent,
    /// The footer text component (displayed below the player list)
    pub footer: EncodedComponent,
}

impl CTabList {
    /// Creates a new tab list packet with the specified header and footer.
    ///
    /// Both must already be resolved for their recipients.
    #[must_use]
    pub fn new(header: impl Into<EncodedComponent>, footer: impl Into<EncodedComponent>) -> Self {
        Self {
            header: header.into(),
            footer: footer.into(),
        }
    }
}

impl steel_utils::serial::WriteTo for CTabList {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.header.write(writer)?;
        self.footer.write(writer)?;
        Ok(())
    }
}
