//! /// Handler for the "give" command.
use std::sync::Arc;

use steel_registry::{data_components::vanilla_components, item_stack::ItemStack, items::ItemRef};
use steel_utils::translations;
use text_components::TextComponent;

use crate::{
    command::{
        arguments::{integer::IntegerArgument, item::ItemStackArgument, player::PlayerArgument},
        commands::{CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument},
        context::CommandContext,
        error::CommandError,
    },
    inventory::container::Container,
    player::Player,
};

/// Handler for the "give" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["give"],
        "Give players the specified item with a specific amount.",
        "minecraft:command.give",
    )
    .then(
        argument("targets", PlayerArgument::multiple()).then(
            argument("item", ItemStackArgument) // FIXME: should be item predicate instead to also handle tags and components
                .executes(GiveNoCountExecutor)
                .then(
                    argument("count", IntegerArgument::bounded(Some(1), None))
                        .executes(GiveWithCountExecutor),
                ),
        ),
    )
}

struct GiveNoCountExecutor;

impl CommandExecutor<(((), Vec<Arc<Player>>), ItemRef)> for GiveNoCountExecutor {
    fn execute(
        &self,
        args: (((), Vec<Arc<Player>>), ItemRef),
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let (((), targets), item) = args;

        let count = give(&targets, item, 1);

        if targets.len() == 1 {
            context.sender.send_message(
                &translations::COMMANDS_GIVE_SUCCESS_SINGLE
                    .message([
                        TextComponent::from(format!("{count}")),
                        TextComponent::from(item.key.path.to_string()),
                        TextComponent::from(
                            targets
                                .first()
                                .expect("targets cannot be empty.")
                                .gameprofile
                                .name
                                .clone(),
                        ),
                    ])
                    .into(),
            );
        } else {
            context.sender.send_message(
                &translations::COMMANDS_GIVE_SUCCESS_MULTIPLE
                    .message([
                        TextComponent::from(format!("{count}")),
                        TextComponent::from(item.key.path.to_string()),
                        TextComponent::from(targets.len().to_string()),
                    ])
                    .into(),
            );
        }

        Ok(())
    }
}

struct GiveWithCountExecutor;

impl CommandExecutor<((((), Vec<Arc<Player>>), ItemRef), i32)> for GiveWithCountExecutor {
    fn execute(
        &self,
        args: ((((), Vec<Arc<Player>>), ItemRef), i32),
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let ((((), targets), item), input_count) = args;

        let actual_count = give(&targets, item, input_count);

        if targets.len() == 1 {
            context.sender.send_message(
                &translations::COMMANDS_GIVE_SUCCESS_SINGLE
                    .message([
                        TextComponent::from(format!("{actual_count}")),
                        TextComponent::from(item.key.path.to_string()),
                        TextComponent::from(
                            targets
                                .first()
                                .expect("targets cannot be empty.")
                                .gameprofile
                                .name
                                .clone(),
                        ),
                    ])
                    .into(),
            );
        } else {
            context.sender.send_message(
                &translations::COMMANDS_GIVE_SUCCESS_MULTIPLE
                    .message([
                        TextComponent::from(format!("{actual_count}")),
                        TextComponent::from(item.key.path.to_string()),
                        TextComponent::from(targets.len().to_string()),
                    ])
                    .into(),
            );
        }

        Ok(())
    }
}

fn give(targets: &Vec<Arc<Player>>, item: ItemRef, count: i32) -> i32 {
    let max_stack_size = item
        .components
        .get(vanilla_components::MAX_STACK_SIZE)
        .unwrap_or(1);

    if count > max_stack_size * 100 {
        return 0;
    }

    let stack = ItemStack::new(item);

    for target in targets {
        let mut remaining = count;

        while remaining > 0 {
            let stack_size = max_stack_size.min(remaining);
            remaining -= stack_size;
            let mut copy = stack.copy_with_count(stack_size);
            let added = target.inventory.lock().add(&mut copy);

            if !added || !copy.is_empty() {
                target.drop_item(copy, false, false);
            }
        }
    }

    targets.len() as i32
}
