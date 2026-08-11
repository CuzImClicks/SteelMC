use glam::DVec3;
use text_components::{EncodedComponent, text_nbt};

use super::{
    Arc, CEntityEvent, CGameEvent, CSystemChat, CTabList, CTickingState, CTickingStep, Color,
    CommandSender, CommandSource, ConnectionProtocol, DisplayResolutor, EncodedPacket, Entity,
    GameEventType, NetworkConnection, Player, Server, SprintReport, TabListTickStats, Uuid,
    client_permission_event, command_tree_packet, translations,
};

const TAB_HEADER: EncodedComponent = text_nbt!("\n<yellow>Steel Dev Build</yellow>\n");

impl Server {
    /// Logs and broadcasts a system chat message to online players.
    fn broadcast_system_chat(&self, message: EncodedComponent, excluded_player: Option<Uuid>) {
        // logging is not worth failing a broadcast over
        match message.to_plain(&DisplayResolutor) {
            Ok(plain) => log::info!("{plain}"),
            Err(error) => log::warn!("broadcast message did not decode for logging: {error}"),
        }
        let Ok(encoded) = EncodedPacket::from_bare(
            CSystemChat::new(message, false),
            self.config.compression,
            ConnectionProtocol::Play,
        ) else {
            return;
        };
        self.online_players.iter_players(|uuid, player| {
            if Some(*uuid) != excluded_player {
                player.connection.send_encoded(encoded.clone());
            }
            true
        });
    }

    /// Builds the tab list header/footer with recent and five-second tick statistics.
    pub(super) fn tab_list_components(
        tick_stats: TabListTickStats,
        movement: DVec3,
    ) -> EncodedComponent {
        // Color TPS based on value
        let tps_color = if tick_stats.tps >= 19.5 {
            Color::Green
        } else if tick_stats.tps >= 15.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        let mspt_color = |mspt: f32| {
            if mspt <= 50.0 {
                Color::Aqua
            } else {
                Color::Red
            }
        };

        let recent_color = mspt_color(tick_stats.recent_mspt);
        let average_color = mspt_color(tick_stats.average_mspt);
        let p95_color = mspt_color(tick_stats.p95_mspt);

        let TabListTickStats {
            tps,
            recent_mspt,
            average_mspt,
            p95_mspt,
        } = tick_stats;

        text_nbt!(
            "\n<gray>TPS: </gray><{tps_color}>{tps:.1}</{tps_color}>\
             <dark_gray> | </dark_gray><gray>MSPT: </gray>\
             <{recent_color}>{recent_mspt:.2}</{recent_color}><gray> recent | </gray>\
             <{average_color}>{average_mspt:.2}</{average_color}><gray> avg (5s) | </gray>\
             <{p95_color}>{p95_mspt:.2}</{p95_color}><gray> p95</gray>\n\
             <gray>Movement: </gray><aqua>{:.2} bps</aqua>\n\
             ",
            movement.with_y(0.0).length() * 20.0,
        )
    }

    /// Broadcasts the tab list header/footer with current TPS and MSPT statistics.
    pub(super) fn broadcast_tab_list(&self, tick_stats: TabListTickStats) {
        self.online_players.iter_players(|_uuid, player| {
            player.send_packet(CTabList::new(
                TAB_HEADER,
                Self::tab_list_components(tick_stats, player.known_movement()),
            ));
            true
        });
    }

    /// Broadcasts a sprint completion report to all players.
    pub(crate) fn broadcast_sprint_report(&self, report: &SprintReport) {
        let message = text_nbt!(
            "<lang:{}:'{}':'{:.2}'>",
            &translations::COMMANDS_TICK_SPRINT_REPORT,
            report.ticks_per_second,
            report.ms_per_tick
        );

        self.broadcast_system_chat(message, None);
    }

    pub(super) fn broadcast_player_join_message(
        &self,
        player: &Player,
        previous_name: Option<&str>,
    ) {
        let display_name = player.display_name();
        // Fallback to the current name when the cache has no prior entry.
        let old_name = previous_name.unwrap_or(player.gameprofile.name.as_str());
        let message = if player.gameprofile.name.eq_ignore_ascii_case(old_name) {
            text_nbt!(
                "<yellow><lang:{}:'{@}'></yellow>",
                &translations::MULTIPLAYER_PLAYER_JOINED,
                display_name
            )
        } else {
            text_nbt!(
                "<yellow><lang:{}:'{@}':'{}'></yellow>",
                &translations::MULTIPLAYER_PLAYER_JOINED_RENAMED,
                display_name,
                old_name
            )
        };
        self.broadcast_system_chat(message, Some(player.gameprofile.id));
    }

    pub(super) fn broadcast_player_leave_message(&self, player: &Player) {
        let message = text_nbt!(
            "<yellow><lang:{}:'{@}'></yellow>",
            &translations::MULTIPLAYER_PLAYER_LEFT,
            player.display_name()
        );
        self.broadcast_system_chat(message, None);
    }

    /// Broadcasts the current tick rate and frozen state to all clients.
    /// This should be called whenever the tick rate or frozen state changes.
    pub fn broadcast_ticking_state(&self) {
        let tick_manager = self.tick_rate_manager.read();
        let packet = CTickingState::new(tick_manager.tick_rate(), tick_manager.is_frozen());
        drop(tick_manager);

        self.broadcast_to_online(packet);
    }

    /// Broadcasts the current step tick count to all clients.
    /// This should be called whenever the step tick count changes.
    pub fn broadcast_ticking_step(&self) {
        let tick_manager = self.tick_rate_manager.read();
        let packet = CTickingStep::new(tick_manager.frozen_ticks_to_run());
        drop(tick_manager);

        self.broadcast_to_online(packet);
    }

    /// Sends the current ticking state and step packets to a joining player.
    /// This should be called when a player joins the server.
    pub fn send_ticking_state_to_player(&self, player: &Player) {
        let tick_manager = self.tick_rate_manager.read();
        let state_packet = CTickingState::new(tick_manager.tick_rate(), tick_manager.is_frozen());
        let step_packet = CTickingStep::new(tick_manager.frozen_ticks_to_run());
        drop(tick_manager);

        player.send_packet(state_packet);
        player.send_packet(step_packet);
    }

    /// Resends client state that is not fully covered by `CRespawn`.
    pub fn resend_player_context(self: &Arc<Self>, player: &Arc<Player>) {
        player.send_difficulty();
        player.send_inventory_to_remote();

        self.resend_player_permission_context(player);

        self.send_ticking_state_to_player(player);

        player.send_packet(CGameEvent {
            event: GameEventType::ChangeGameMode,
            data: player.game_mode().into(),
        });
    }

    /// Resends the command tree and vanilla client permission-level projection.
    pub fn resend_player_permission_context(self: &Arc<Self>, player: &Arc<Player>) {
        let world = player.get_world();
        player.send_packet(CEntityEvent {
            entity_id: player.id(),
            event: client_permission_event(player, &world),
        });

        let server = player.server();
        if !Arc::ptr_eq(&server, self) {
            tracing::error!(
                player = %player.gameprofile.name,
                "cannot project commands from a different server"
            );
            return;
        }
        let Some(shared_player) = self.online_players.get_by_uuid(&player.gameprofile.id) else {
            tracing::error!(
                player = %player.gameprofile.name,
                "cannot project commands for a player outside the online player map"
            );
            return;
        };
        if !Arc::ptr_eq(&shared_player, player) {
            tracing::error!(
                player = %player.gameprofile.name,
                "cannot project commands for a stale player handle"
            );
            return;
        }
        let source = CommandSource::new(CommandSender::Player(shared_player), server);
        let commands = {
            let dispatcher = self.command_dispatcher.read();
            command_tree_packet(&dispatcher, &source)
        };
        match commands {
            Ok(commands) => player.send_packet(commands),
            Err(error) => tracing::error!(
                player = %player.gameprofile.name,
                %error,
                "failed to project the player's command tree"
            ),
        }
    }
}
