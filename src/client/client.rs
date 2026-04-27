use crate::protocol::protocol::*;
use crate::shared::shared::*;
use avian3d::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input::keyboard::Key;
use bevy::prelude::*;
use core::time::Duration;
use lightyear::prelude::Controlled;
use lightyear::prelude::client::*;
use lightyear::prelude::input::InputBuffer;
use lightyear::prelude::input::bei::InputMarker;
use lightyear::prelude::input::bei::TriggerState;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ClientTransports {
    #[cfg(not(target_family = "wasm"))]
    Udp,
    WebTransport,
    WebSocket,
}

#[derive(Component, Clone, Debug)]
#[component(on_add = PlayerClient::on_add)]
pub struct PlayerClient {
    pub client_id: u64,
    /// The client port to listen on
    pub client_port: u16,
    /// The socket address of the server
    pub server_addr: SocketAddr,
    /// Possibly add a conditioner to simulate network conditions
    pub conditioner: Option<RecvLinkConditioner>,
    /// Which transport to use
    pub transport: ClientTransports,

    pub shared: SharedSettings,
}

impl PlayerClient {
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let entity = context.entity;
        world.commands().queue(move |world: &mut World| -> Result {
            let mut entity_mut = world.entity_mut(entity);
            let settings = entity_mut.take::<PlayerClient>().unwrap();
            let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), settings.client_port);
            entity_mut.insert((
                Client::default(),
                Link::new(settings.conditioner.clone()),
                LocalAddr(client_addr),
                PeerAddr(settings.server_addr),
                ReplicationReceiver::default(),
                PredictionManager::default(),
                Name::from("Client"),
            ));
            Ok(())
        });
    }
}

pub struct PlayerClientPlugin;

impl Plugin for PlayerClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_new_floor, handle_new_block, handle_new_character),
        );
    }
}

/// Add physics to characters that are newly predicted. If the client controls
/// the character then add an input component.
fn handle_new_character(
    mut commands: Commands,
    mut character_query: Query<
        (Entity, &ColorComponent, Has<Controlled>),
        (Added<Predicted>, With<CharacterMarker>),
    >,
) {
    for (entity, _color, is_controlled) in &mut character_query {
        if is_controlled {
            info!("Adding InputMap to controlled and predicted entity {entity:?}");
            commands.entity(entity);
        } else {
            info!("Remote character predicted for us: {entity:?}");
        }
        info!(?entity, "Adding physics to character");
        commands
            .entity(entity)
            .insert(CharacterPhysicsBundle::default());
    }
}

/// Add physics to floors that are newly replicated. The query checks for
/// replicated floors instead of predicted floors because predicted floors do
/// not exist since floors aren't predicted.
fn handle_new_floor(
    mut commands: Commands,
    floor_query: Query<Entity, (Added<Replicated>, With<FloorMarker>)>,
) {
    for entity in &floor_query {
        info!(?entity, "Adding physics to floor");
        commands
            .entity(entity)
            .insert(FloorPhysicsBundle::default());
    }
}

/// Add physics to blocks that are newly predicted.
fn handle_new_block(
    mut commands: Commands,
    block_query: Query<Entity, (Added<Predicted>, With<BlockMarker>)>,
) {
    for entity in &block_query {
        info!(?entity, "Adding physics to block");
        commands
            .entity(entity)
            .insert(BlockPhysicsBundle::default());
    }
}

pub(crate) fn connect(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}
