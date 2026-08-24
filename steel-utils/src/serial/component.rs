//! Pre-encoded text components.

use std::io::{Result, Write};

use text_components::EncodedComponent;

use crate::serial::WriteTo;

impl WriteTo for EncodedComponent {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(self.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use text_components::{EncodedComponent, Modifier, Style, TextComponent, format::Color};

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

        let mut from_encoded = Vec::new();
        component
            .encode()
            .write(&mut from_encoded)
            .expect("writing to a Vec cannot fail");

        assert_eq!(from_tree, from_encoded);
    }

    #[test]
    fn static_bytes_are_written_verbatim() {
        static BYTES: &[u8] = &[0x0A, 0x08, 0x00, 0x04, b't', b'e', b'x', b't', 0x00];
        let encoded = EncodedComponent::from_static(BYTES);

        let mut out = Vec::new();
        encoded
            .write(&mut out)
            .expect("writing to a Vec cannot fail");

        assert_eq!(out, BYTES);
        assert_eq!(encoded.as_bytes(), BYTES);
    }
}
