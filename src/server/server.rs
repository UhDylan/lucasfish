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
    Udp {
        local_port: u16,
    },
    #[cfg(feature = "steam")]
    Steam {
        local_port: u16,
    },
}

#[derive(Clone)]
pub struct CoreServerPlugin;

impl Plugin for CoreServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_observer(handle_new_client);
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
                ServerTransports::Udp { local_port } => {
                    add_netcode(&mut entity_mut);
                    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
                    entity_mut.insert((LocalAddr(server_addr), ServerUdpIo::default()));
                }
                #[cfg(feature = "steam")]
                ServerTransports::Steam { local_port } => {
                    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
                    entity_mut.insert(SteamServerIo {
                        target: ListenTarget::Addr(server_addr),
                        config: SessionConfig::default(),
                    });
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

fn handle_new_client(trigger: On<Add, LinkOf>) {
    info!("Client connected: {:?}", trigger.event_target());
}
