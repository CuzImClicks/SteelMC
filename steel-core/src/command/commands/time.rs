use steel_protocol::packets::game::CSetTime;
use steel_registry::vanilla_game_rules::ADVANCE_TIME;
use steel_utils::translations;
use text_components::TextComponent;

use crate::command::{
    arguments::time::TimeArgument,
    commands::{CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument, literal},
    context::CommandContext,
    error::CommandError,
};

#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["time"],
        "Allows interacting with the ingame time.",
        "minecraft:command.time",
    )
    .then(
        literal("query")
            .then(literal("day").executes(TimeQueryDayExecutor))
            .then(literal("daytime").executes(TimeQueryDayTimeExecutor))
            .then(literal("gametime").executes(TimeQueryGameTimeExecutor)),
    )
    .then(
        literal("set")
            .then(literal("day").executes(TimeConstSetExecutor::<1000>))
            .then(literal("midnight").executes(TimeConstSetExecutor::<18000>))
            .then(literal("night").executes(TimeConstSetExecutor::<13000>))
            .then(literal("noon").executes(TimeConstSetExecutor::<6000>))
            .then(argument("time", TimeArgument).executes(TimeSetExecutor)),
    )
    .then(literal("add").then(argument("time", TimeArgument).executes(TimeAddExecutor)))
}

struct TimeQueryDayExecutor;

impl CommandExecutor<()> for TimeQueryDayExecutor {
    fn execute(&self, _args: (), context: &mut CommandContext) -> Result<(), CommandError> {
        let day = context.world.level_data.read().day();
        context.sender.send_message(
            &translations::COMMANDS_TIME_QUERY
                .message([TextComponent::from(format!("{day}"))])
                .into(),
        );
        Ok(())
    }
}

struct TimeQueryDayTimeExecutor;

impl CommandExecutor<()> for TimeQueryDayTimeExecutor {
    fn execute(&self, _args: (), context: &mut CommandContext) -> Result<(), CommandError> {
        let day_time = context.world.level_data.read().day_time();
        context.sender.send_message(
            &translations::COMMANDS_TIME_QUERY
                .message([TextComponent::from(format!("{day_time}"))])
                .into(),
        );
        Ok(())
    }
}

struct TimeQueryGameTimeExecutor;

impl CommandExecutor<()> for TimeQueryGameTimeExecutor {
    fn execute(&self, _args: (), context: &mut CommandContext) -> Result<(), CommandError> {
        let game_time = context.world.level_data.read().game_time();
        context.sender.send_message(
            &translations::COMMANDS_TIME_QUERY
                .message([TextComponent::from(format!("{game_time}"))])
                .into(),
        );
        Ok(())
    }
}

struct TimeAddExecutor;

impl CommandExecutor<((), i32)> for TimeAddExecutor {
    fn execute(&self, args: ((), i32), context: &mut CommandContext) -> Result<(), CommandError> {
        let mut day_time_option: Option<i64> = None;

        context.server.worlds.iter().for_each(|world| {
            let (game_time, new_day_time) = {
                let mut lock = world.level_data.write();

                let game_time = lock.game_time();
                let new_day_time = (lock.day_time() + i64::from(args.1)) % 24000;

                lock.set_day_time(new_day_time);
                (game_time, new_day_time)
            };

            let advance_time = { world.get_game_rule(ADVANCE_TIME).as_bool().expect("todo") };

            day_time_option = Some(new_day_time);

            world.broadcast_to_all(CSetTime::new(game_time, new_day_time, advance_time));
        });

        let Some(new_day_time) = day_time_option else {
            return Err(CommandError::CommandFailed(Box::new(TextComponent::from(
                "no world to update time on",
            ))));
        };

        context.sender.send_message(
            &translations::COMMANDS_TIME_SET
                .message([TextComponent::from(format!("{new_day_time}"))])
                .into(),
        );

        Ok(())
    }
}

struct TimeConstSetExecutor<const DAYTIME: i64>;

impl<const DAYTIME: i64> CommandExecutor<()> for TimeConstSetExecutor<DAYTIME> {
    fn execute(&self, _args: (), context: &mut CommandContext) -> Result<(), CommandError> {
        context.server.worlds.iter().for_each(|world| {
            let (game_time, new_day_time) = {
                let mut lock = world.level_data.write();

                let game_time = lock.game_time();
                let new_day_time = DAYTIME;

                lock.set_day_time(new_day_time);
                (game_time, new_day_time)
            };

            let advance_time = { world.get_game_rule(ADVANCE_TIME).as_bool().expect("todo") };

            world.broadcast_to_all(CSetTime::new(game_time, new_day_time, advance_time));
        });

        context.sender.send_message(
            &translations::COMMANDS_TIME_SET
                .message([TextComponent::from(format!("{DAYTIME}"))])
                .into(),
        );

        Ok(())
    }
}

struct TimeSetExecutor;

impl CommandExecutor<((), i32)> for TimeSetExecutor {
    fn execute(&self, args: ((), i32), context: &mut CommandContext) -> Result<(), CommandError> {
        context.server.worlds.iter().for_each(|world| {
            let (game_time, new_day_time) = {
                let mut lock = world.level_data.write();

                let game_time = lock.game_time();
                let new_day_time = i64::from(args.1);

                lock.set_day_time(new_day_time);
                (game_time, new_day_time)
            };

            let advance_time = { world.get_game_rule(ADVANCE_TIME).as_bool().expect("todo") };

            world.broadcast_to_all(CSetTime::new(game_time, new_day_time, advance_time));
        });

        context.sender.send_message(
            &translations::COMMANDS_TIME_SET
                .message([TextComponent::from(format!("{}", args.1))])
                .into(),
        );

        Ok(())
    }
}
