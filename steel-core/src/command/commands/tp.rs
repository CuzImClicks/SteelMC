//! Handler for the "teleport" command.
use std::sync::Arc;

use steel_utils::{math::Vector3, translations};
use text_components::TextComponent;

use crate::{
    command::{
        arguments::{player::PlayerArgument, rotation::RotationArgument, vector3::Vector3Argument},
        commands::{CommandHandlerBuilder, CommandHandlerDyn, argument},
        context::CommandContext,
        error::CommandError,
        sender::CommandSender,
    },
    entity::Entity,
    player::Player,
};

type MultipleRotationArgs = ((((), Vec<Arc<Player>>), Vector3<f64>), (f32, f32));
type MultipleEntityArgs = (((), Vec<Arc<Player>>), Vec<Arc<Player>>);

/// Handler for the "teleport" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["tp", "teleport"],
        "Teleports the target(s) to the given location.",
        "minecraft:command.teleport",
    )
    .then(
        argument("targets", PlayerArgument::multiple())
            .then(
                argument("position", Vector3Argument)
                    .executes(
                        |(((), targets), pos): (((), Vec<Arc<Player>>), Vector3<f64>),
                         context: &mut CommandContext| {
                            let player = context
                                .sender
                                .get_player()
                                .ok_or(CommandError::InvalidRequirement)?;

                            teleport_to_pos(&targets, pos, player.rotation(), &context.sender);

                            Ok(())
                        },
                    )
                    .then(argument("rotation", RotationArgument).executes(
                        |((((), targets), pos), rotation): MultipleRotationArgs,
                         context: &mut CommandContext| {
                            teleport_to_pos(&targets, pos, rotation, &context.sender);

                            Ok(())
                        },
                    )),
            )
            .then(argument("destination", PlayerArgument::one()).executes(
                |(((), targets), destination): MultipleEntityArgs, context: &mut CommandContext| {
                    teleport_to_player(&targets, &destination, &context.sender);

                    Ok(())
                },
            )),
    )
    .then(
        argument("location", Vector3Argument)
            .executes(|((), pos), context: &mut CommandContext| {
                let player = context
                    .player
                    .clone()
                    .ok_or(CommandError::InvalidRequirement)?;
                let rotation = player.rotation();

                teleport_to_pos(&[player], pos, rotation, &context.sender);

                Ok(())
            })
            .then(argument("rotation", RotationArgument).executes(
                |(((), pos), rotation), context: &mut CommandContext| {
                    let player = context
                        .player
                        .clone()
                        .ok_or(CommandError::InvalidRequirement)?;

                    teleport_to_pos(&[player], pos, rotation, &context.sender);
                    Ok(())
                },
            )),
    )
}

fn teleport_to_pos(
    targets: &[Arc<Player>],
    pos: Vector3<f64>,
    rotation: (f32, f32),
    sender: &CommandSender,
) {
    for player in targets {
        player.teleport(pos.x, pos.y, pos.z, rotation.0, rotation.1);
    }

    if targets.len() == 1 {
        sender.send_message(
            &translations::COMMANDS_TELEPORT_SUCCESS_LOCATION_SINGLE
                .message([
                    TextComponent::from(
                        targets
                            .first()
                            .expect("targets cannot be empty")
                            .gameprofile
                            .name
                            .clone(),
                    ),
                    TextComponent::from(format!("{:.2}", pos.x)),
                    TextComponent::from(format!("{:.2}", pos.y)),
                    TextComponent::from(format!("{:.2}", pos.z)),
                ])
                .into(),
        );
    } else {
        sender.send_message(
            &translations::COMMANDS_TELEPORT_SUCCESS_LOCATION_MULTIPLE
                .message([
                    TextComponent::from(format!("{}", targets.len())),
                    TextComponent::from(format!("{:.2}", pos.x)),
                    TextComponent::from(format!("{:.2}", pos.y)),
                    TextComponent::from(format!("{:.2}", pos.z)),
                ])
                .into(),
        );
    }
}

fn teleport_to_player(
    targets: &[Arc<Player>],
    destination: &[Arc<Player>],
    sender: &CommandSender,
) {
    let destination = destination
        .first()
        .expect("destination should not be empty");

    let pos = destination.position();
    let (yaw, pitch) = destination.rotation();

    for player in targets {
        player.teleport(pos.x, pos.y, pos.z, yaw, pitch);
    }

    if targets.len() == 1 {
        sender.send_message(
            &translations::COMMANDS_TELEPORT_SUCCESS_ENTITY_SINGLE
                .message([
                    TextComponent::from(
                        targets
                            .first()
                            .expect("targets cannot be empty")
                            .gameprofile
                            .name
                            .clone(),
                    ),
                    TextComponent::from(destination.gameprofile.name.clone()),
                ])
                .into(),
        );
    } else {
        sender.send_message(
            &translations::COMMANDS_TELEPORT_SUCCESS_ENTITY_MULTIPLE
                .message([
                    TextComponent::from(format!("{}", targets.len())),
                    TextComponent::from(destination.gameprofile.name.clone()),
                ])
                .into(),
        );
    }
}
