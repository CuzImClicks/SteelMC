//! Handler for the "clear" command.
use std::sync::Arc;

use steel_utils::translations;
use text_components::TextComponent;

use crate::{
    command::{
        arguments::player::PlayerArgument,
        commands::{CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument},
        context::CommandContext,
        error::CommandError,
    },
    inventory::container::Container,
    player::Player,
};

/// Handler for the "clear" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["clear"],
        "Clears the Player's inventory.",
        "minecraft:command.clear",
    )
    .executes(ClearNoArgumentExecutor)
    .then(
        argument("targets", PlayerArgument::multiple()).executes(ClearMultipleArgumentExecutor),
        // should be an EntityArgument
        //.then(argument("item", ItemArgument) // with tags somehow
        //    .then(argument("maxCount", IntegerArgument::bounded(0, None)))
        //)
    )
}

struct ClearNoArgumentExecutor;

impl CommandExecutor<()> for ClearNoArgumentExecutor {
    fn execute(&self, _args: (), context: &mut CommandContext) -> Result<(), CommandError> {
        let player = context
            .sender
            .get_player()
            .ok_or(CommandError::InvalidRequirement)?;

        let count = { player.inventory.lock().clear_content() };

        if count == 0 {
            context.sender.send_message(
                &translations::CLEAR_FAILED_SINGLE
                    .message([TextComponent::from(player.gameprofile.name.clone())])
                    .into(),
            );
            return Ok(());
        }

        context.sender.send_message(
            &translations::COMMANDS_CLEAR_SUCCESS_SINGLE
                .message([
                    TextComponent::from(format!("{count}")),
                    TextComponent::from(player.gameprofile.name.clone()),
                ])
                .into(),
        );

        Ok(())
    }
}

struct ClearMultipleArgumentExecutor;

impl CommandExecutor<((), Vec<Arc<Player>>)> for ClearMultipleArgumentExecutor {
    fn execute(
        &self,
        args: ((), Vec<Arc<Player>>),
        context: &mut CommandContext,
    ) -> Result<(), CommandError> {
        let ((), targets) = args;

        let count: i32 = targets
            .iter()
            .map(|it| it.inventory.lock().clear_content())
            .sum();

        if count == 0 {
            context.sender.send_message(
                &translations::CLEAR_FAILED_MULTIPLE
                    .message([TextComponent::from(format!("{}", targets.len()))])
                    .into(),
            );
        } else {
            context.sender.send_message(
                &translations::COMMANDS_CLEAR_SUCCESS_MULTIPLE
                    .message([
                        TextComponent::from(format!("{count}")),
                        TextComponent::from(format!("{}", targets.len())),
                    ])
                    .into(),
            );
        }

        Ok(())
    }
}
