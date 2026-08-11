//! Pre-encoded text components.

use std::{
    borrow::Cow,
    io::{Result, Write},
};

use text_components::{EncodedComponent, TextComponent};

use crate::serial::WriteTo;

/// A text component already encoded as network NBT (`[tag type][payload]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawComponent(Cow<'static, [u8]>);

impl RawComponent {
    /// Wraps bytes from a fully static `text_nbt!`.
    #[must_use]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    /// Encodes a component tree.
    #[must_use]
    pub fn encode(component: &TextComponent) -> Self {
        let mut buf = Vec::with_capacity(128);
        component.to_codec_nbt().write(&mut buf);
        Self(Cow::Owned(buf))
    }

    /// The encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<&TextComponent> for RawComponent {
    fn from(component: &TextComponent) -> Self {
        Self::encode(component)
    }
}

impl From<TextComponent> for RawComponent {
    fn from(component: TextComponent) -> Self {
        Self::encode(&component)
    }
}

impl From<EncodedComponent> for RawComponent {
    fn from(encoded: EncodedComponent) -> Self {
        Self(encoded.into_bytes())
    }
}

impl WriteTo for RawComponent {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use text_components::{Modifier, Style, TextComponent, format::Color};

    use super::RawComponent;
    use crate::serial::WriteTo;

    #[test]
    fn encoding_matches_writing_the_tree_directly() {
        let component = TextComponent::plain("hello")
            .color(Color::Gold)
            .add_children(vec![TextComponent::plain(" world")]);

        let mut from_tree = Vec::new();
        component
            .write(&mut from_tree)
            .expect("writing to a Vec cannot fail");

        let mut from_raw = Vec::new();
        RawComponent::encode(&component)
            .write(&mut from_raw)
            .expect("writing to a Vec cannot fail");

        assert_eq!(from_tree, from_raw);
    }

    #[test]
    fn static_bytes_are_written_verbatim() {
        static BYTES: &[u8] = &[0x0A, 0x08, 0x00, 0x04, b't', b'e', b'x', b't', 0x00];
        let raw = RawComponent::from_static(BYTES);

        let mut out = Vec::new();
        raw.write(&mut out).expect("writing to a Vec cannot fail");

        assert_eq!(out, BYTES);
        assert_eq!(raw.as_bytes(), BYTES);
    }
}
