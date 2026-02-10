#![expect(missing_docs)]

use steel_utils::Identifier;
use winnow::{
    Parser, Result,
    ascii::multispace0,
    combinator::{alt, delimited, opt},
    error::{ParserError, StrContext},
    token::take_while,
};

pub mod block;

pub fn parse_identifier(input: &mut &str) -> winnow::Result<Identifier> {
    dbg!(&input);
    let first = parse_namespace
        .context(StrContext::Label("Failed to parse namespace"))
        .parse_next(input)?;

    if opt(":").parse_next(input)?.is_some() {
        let path = parse_path(input)?;
        Ok(Identifier::new(first.to_string(), path.to_string()))
    } else {
        Ok(Identifier::new("minecraft", first.to_string()))
    }
}

pub fn parse_namespace<'s>(input: &mut &'s str) -> Result<&'s str> {
    take_while(1.., Identifier::valid_namespace_char).parse_next(input)
}

pub fn parse_path<'s>(input: &mut &'s str) -> Result<&'s str> {
    take_while(1.., Identifier::valid_char).parse_next(input)
}

pub fn parse_identifier_str<'s>(input: &mut &'s str) -> winnow::Result<&'s str> {
    take_while(.., (('a'..='z'), '_')).parse_next(input)
}

pub fn parse_bool(input: &mut &str) -> winnow::Result<bool> {
    alt((ws("true"), ws("false"))).parse_to().parse_next(input)
}

pub fn ws<'i, O, E, P>(inner: P) -> impl Parser<&'i str, O, E>
where
    P: Parser<&'i str, O, E>,
    E: ParserError<&'i str>,
{
    delimited(multispace0, inner, multispace0)
}

#[cfg(test)]
mod tests {
    use crate::block::BlockArgument;
    use crate::block::parse_block_state_argument;
    use steel_registry::REGISTRY;
    use steel_registry::{Registry, vanilla_blocks};

    use super::*;

    fn init_test_registries() {
        REGISTRY.get_or_init(|| {
            let mut registry = Registry::new_vanilla();
            registry.freeze();
            registry
        });
    }

    #[test]
    fn parses_block_and_boolean() {
        init_test_registries();
        let mut input = "cake[] true false";
        let BlockArgument {
            block,
            parsed_properties,
        } = parse_block_state_argument(&mut input).unwrap();
        let boolean = parse_bool(&mut input).unwrap();
        let boolean2 = parse_bool(&mut input).unwrap();
        assert!(block.key == vanilla_blocks::CAKE.key);
        assert!(parsed_properties.is_some());
        assert!(
            boolean,
            "parsed boolean should have been true but was '{}'",
            boolean
        );
        assert!(
            !boolean2,
            "parsed boolean2 should have been false but was '{}'",
            boolean2
        );
    }

    #[test]
    fn parses_multiple_identifiers() -> Result<()> {
        init_test_registries();
        let mut input = "minecraft:azalea steel:flint flint_and_steel";
        let first = ws(parse_identifier).parse_next(&mut input)?;

        let second = ws(parse_identifier).parse_next(&mut input)?;
        let third = ws(parse_identifier).parse_next(&mut input)?;
        assert!(
            first.namespace == "minecraft" && first.path == "azalea",
            "minecraft | {} - azalea | {}",
            first.namespace,
            first.path
        );
        assert!(
            second.namespace == "steel" && second.path == "flint",
            "steel | {} - flint | {}",
            second.namespace,
            second.path
        );
        assert!(
            third.namespace == "minecraft" && third.path == "flint_and_steel",
            "minecraft | {} - flint_and_steel | {}",
            third.namespace,
            third.path
        );
        Ok(())
    }
}
