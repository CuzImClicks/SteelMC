use steel_registry::{
    REGISTRY,
    blocks::{BlockRef, properties::DynProperty},
};
use winnow::{
    Parser, Result,
    combinator::{delimited, peek, separated},
    error::{ContextError, ParserError},
    token::{literal, take_till},
};

use crate::{parse_identifier, parse_identifier_str, ws};

pub struct BlockArgument<'s> {
    pub block: BlockRef,
    pub parsed_properties: Option<Vec<(&'static dyn DynProperty, &'s str)>>,
}

pub fn parse_block_state_argument<'s>(input: &mut &'s str) -> winnow::Result<BlockArgument<'s>> {
    let block_name = parse_identifier(input)?;
    let block_ref = REGISTRY
        .blocks
        .by_key(&block_name)
        .ok_or(ParserError::from_input(input))?;
    let properties = if peek::<_, _, ContextError, _>("[").parse_next(input).is_ok() {
        Some(parse_properties(input, block_ref.properties)?)
    } else {
        None
    };
    Ok(BlockArgument {
        block: block_ref,
        parsed_properties: properties,
    })
}

pub fn parse_properties<'s>(
    input: &mut &'s str,
    allowed: &'static [&'static dyn DynProperty],
) -> winnow::Result<Vec<(&'static dyn DynProperty, &'s str)>> {
    delimited(
        "[",
        separated(0.., |i: &mut &'s str| parse_property(i, allowed), ","),
        "]",
    )
    .parse_next(input)
}

pub fn parse_property<'s>(
    input: &mut &'s str,
    allowed: &'static [&'static dyn DynProperty],
) -> Result<(&'static dyn DynProperty, &'s str)> {
    let property = parse_property_key(input, allowed)?;
    ws(literal("=")).void().parse_next(input)?;
    let value = ws(take_till(1.., |c| c == ',' || c == ']')).parse_next(input)?;
    property
        .get_possible_values()
        .iter()
        .copied()
        .any(|s| value == s)
        .then_some((property, value))
        .ok_or(ParserError::from_input(input))
}

pub fn parse_property_key(
    input: &mut &str,
    allowed: &'static [&'static dyn DynProperty],
) -> winnow::Result<&'static dyn DynProperty> {
    //let checkpoint = input.checkpoint();
    let key = ws(parse_identifier_str).parse_next(input)?;

    allowed
        .iter()
        .copied()
        .find(|p| p.get_name() == key)
        .ok_or(ParserError::from_input(input))

    //input.reset(&checkpoint);

    //let suggestions = allowed.iter().map(|p| p.get_name())
}

#[cfg(test)]
mod tests {
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
    fn parses_block() {
        init_test_registries();
        let mut input = "minecraft:acacia_door";
        let BlockArgument {
            block,
            parsed_properties,
        } = parse_block_state_argument(&mut input).unwrap();
        //println!("{:?} \n\n{:?}", &block, &properties);
        assert!(block.key == vanilla_blocks::ACACIA_DOOR.key);
        assert!(parsed_properties.is_none());
    }

    #[test]
    fn parses_block_with_properties() {
        init_test_registries();
        let mut input = "birch_fence_gate[ facing =east, open= true]";
        let BlockArgument {
            block,
            parsed_properties,
        } = parse_block_state_argument(&mut input).unwrap();
        //println!("{:?} \n\n{:?}", &block, &properties);
        assert!(block.key == vanilla_blocks::BIRCH_FENCE_GATE.key);
        assert!(parsed_properties.is_some());
    }

    #[test]
    fn parses_block_with_empty_properties() {
        init_test_registries();
        let mut input = "cake[]";
        let BlockArgument {
            block,
            parsed_properties,
        } = parse_block_state_argument(&mut input).unwrap();
        //println!("{:?} \n\n{:?}", &block, &properties);
        assert!(block.key == vanilla_blocks::CAKE.key);
        assert!(parsed_properties.is_some());
    }

    #[test]
    fn parses_block_with_false_properties() {
        init_test_registries();
        let mut input = "birch_fence_gate[ facing =east, waterlogged=true]";
        assert!(parse_block_state_argument(&mut input).is_err());
    }
}
