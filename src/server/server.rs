use crate::protocol::protocol::*;
use crate::shared::shared;
use crate::shared::shared::BlockPhysicsBundle;
use crate::shared::shared::CHARACTER_CAPSULE_HEIGHT;
use crate::shared::shared::CHARACTER_CAPSULE_RADIUS;
use crate::shared::shared::CharacterPhysicsBundle;
use crate::shared::shared::FLOOR_HEIGHT;
use crate::shared::shared::FLOOR_WIDTH;
use crate::shared::shared::FloorPhysicsBundle;
use crate::shared::shared::SEND_INTERVAL;
use crate::shared::shared::SharedSettings;
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::math::VectorSpace;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use core::f32::consts::TAU;
use core::net::{Ipv4Addr, SocketAddr};
use core::time::Duration;
use lightyear::connection::client::Connected;
use lightyear::netcode::{NetcodeServer, PRIVATE_KEY_BYTES};
use lightyear::prelude::input::bei::TriggerState;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ServerTransports {
    #[cfg(feature = "server")]
    Udp { local_port: u16 },
}

#[derive(Clone)]
pub struct CoreServerPlugin;

impl Plugin for CoreServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(
            FixedUpdate,
            (handle_character_actions, player_shoot, despawn_system),
        );
        app.add_observer(handle_new_client);
        app.add_observer(handle_connected);
        app.add_plugins(AssetPlugin::default());
    }
}

#[derive(Component, Debug)]
#[component(on_add = CoreServer::on_add)]
pub struct CoreServer {
    /// Possibly add a conditioner to simulate network conditions
    pub conditioner: Option<RecvLinkConditioner>,
    /// Which transport to use
    pub transport: ServerTransports,
    pub shared: SharedSettings,
}

impl CoreServer {
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let entity = context.entity;
        world.commands().queue(move |world: &mut World| -> Result {
            let mut entity_mut = world.entity_mut(entity);
            let settings = entity_mut.take::<CoreServer>().unwrap();
            entity_mut.insert((Name::from("Server"),));

            let add_netcode = |entity_mut: &mut EntityWorldMut| {
                let private_key = if let Some(key) = parse_private_key_from_env() {
                    info!("Using private key from LIGHTYEAR_PRIVATE_KEY env var");
                    key
                } else {
                    settings.shared.private_key
                };
                entity_mut.insert(NetcodeServer::new(NetcodeConfig {
                    protocol_id: settings.shared.protocol_id,
                    private_key,
                    ..Default::default()
                }));
            };
            match settings.transport {
                #[cfg(feature = "server")]
                ServerTransports::Udp { local_port } => {
                    add_netcode(&mut entity_mut);
                    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
                    entity_mut.insert((LocalAddr(server_addr), ServerUdpIo::default()));
                }
            };
            Ok(())
        });
    }
}

pub(crate) fn start(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}

pub fn parse_private_key_from_env() -> Option<[u8; PRIVATE_KEY_BYTES]> {
    let Ok(key_str) = std::env::var("LIGHTYEAR_PRIVATE_KEY") else {
        return None;
    };
    let private_key: Vec<u8> = key_str
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .split(',')
        .map(|s| {
            s.parse::<u8>()
                .expect("Failed to parse number in private key")
        })
        .collect();

    if private_key.len() != PRIVATE_KEY_BYTES {
        panic!("Private key must contain exactly {PRIVATE_KEY_BYTES} numbers",);
    }

    let mut bytes = [0u8; PRIVATE_KEY_BYTES];
    bytes.copy_from_slice(&private_key);
    Some(bytes)
}

fn handle_character_actions(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut query: Query<(Entity, &ComputedMass, &TriggerState, Forces)>,
) {
    for (entity, mass, action_state, forces) in &mut query {
        todo!();
    }
}

#[derive(Component)]
pub struct DespawnAfter {
    spawned_at: f32,
    lifetime: Duration,
}

fn despawn_system(
    mut commands: Commands,
    query: Query<(Entity, &DespawnAfter)>,
    time: Res<Time<Fixed>>,
) {
    for (entity, despawn) in &query {
        if time.elapsed_secs() - despawn.spawned_at >= despawn.lifetime.as_secs_f32() {
            commands.entity(entity).despawn();
        }
    }
}

fn player_shoot(
    commands: Commands,
    timeline: Res<LocalTimeline>,
    query: Query<(&TriggerState, &Position, &ControlledBy), Without<Predicted>>,
    time: Res<Time<Fixed>>,
) {
    for (action_state, position, controlled_by) in &query {
        let mut position_override = ComponentReplicationOverrides::<Position>::default();
        position_override.global_override(ComponentReplicationOverride {
            replicate_once: true,
            ..default()
        });
        let mut rotation_override = ComponentReplicationOverrides::<Rotation>::default();
        rotation_override.global_override(ComponentReplicationOverride {
            replicate_once: true,
            ..default()
        });
        let mut linear_velocity_override =
            ComponentReplicationOverrides::<LinearVelocity>::default();
        linear_velocity_override.global_override(ComponentReplicationOverride {
            replicate_once: true,
            ..default()
        });
        let mut angular_velocity_override =
            ComponentReplicationOverrides::<AngularVelocity>::default();
        angular_velocity_override.global_override(ComponentReplicationOverride {
            replicate_once: true,
            ..default()
        });
        let mut computed_mass_override = ComponentReplicationOverrides::<ComputedMass>::default();
        computed_mass_override.global_override(ComponentReplicationOverride {
            replicate_once: true,
            ..default()
        });
    }
}

// Renamed from init, removed start_server
fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Floor"),
        FloorPhysicsBundle::default(),
        FloorMarker,
        Position::new(Vec3::ZERO),
        Replicate::to_clients(NetworkTarget::All),
    ));

    commands.spawn((
        Name::new("Block"),
        BlockPhysicsBundle::default(),
        BlockMarker,
        Position::new(Vec3::new(1.0, 1.0, 0.0)),
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::All),
    ));
}

/// Add the ReplicationSender component to new clients
pub(crate) fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(ReplicationSender::new(
            SEND_INTERVAL,
            SendUpdatesMode::SinceLastAck,
            false,
        ));
}

/// Spawn the player entity when a client connects
pub(crate) fn handle_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
    character_query: Query<Entity>,
) {
    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = client_id.0;
    info!("Client connected with client-id {client_id:?}. Spawning character entity.");

    // Track the number of characters to pick colors and starting positions.
    let num_characters = character_query.iter().count();

    // Pick color and position for player.
    let available_colors = [
        css::LIMEGREEN,
        css::PINK,
        css::YELLOW,
        css::AQUA,
        css::CRIMSON,
        css::GOLD,
        css::ORANGE_RED,
        css::SILVER,
        css::SALMON,
        css::YELLOW_GREEN,
        css::WHITE,
        css::RED,
    ];
    let color = available_colors[num_characters % available_colors.len()];
    let angle: f32 = num_characters as f32 * 5.0;
    let x = 2.0 * angle.cos();
    let z = 2.0 * angle.sin();

    // Spawn the character with ActionState. The client will add their own InputMap.
    let character = commands
        .spawn((
            Name::new("Character"),
            Position(Vec3::new(x, 3.0, z)),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::All),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
            CharacterPhysicsBundle::default(),
        ))
        .id();

    info!("Created entity {character:?} for client {client_id:?}");
}
