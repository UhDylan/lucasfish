#![allow(unused_variables)]
#![allow(dead_code)]
use crate::client::client::ClientTransports;
use crate::client::client::PlayerClient;
use crate::client::client::connect;
use crate::client_renderer::ClientRendererPlugin;
use crate::server::server::CoreServer;
use crate::server::server::ServerTransports;
use crate::server::server::start;
use crate::shared::shared::*;
use avian3d::PhysicsPlugins;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::ecs::system::command;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_steamworks::*;
use clap::{Parser, Subcommand};
use core::time::Duration;
use lightyear::link::RecvLinkConditioner;
use lightyear::prelude::LinkConditionerConfig;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Option<Mode>,
}

impl Cli {
    pub fn client_id(&self) -> Option<u64> {
        match &self.mode {
            #[cfg(feature = "client")]
            Some(Mode::Client { client_id }) => *client_id,
            _ => None,
        }
    }

    pub fn create_app() -> App {
        #[cfg(feature = "client")]
        let app = client_app();
        #[cfg(feature = "server")]
        let app = server_app();
        app
    }

    pub fn build_app(&self, tick_duration: Duration, add_inspector: bool) -> App {
        let mut app = Cli::create_app();
        match self.mode {
            #[cfg(feature = "client")]
            Some(Mode::Client { client_id }) => {
                app.add_plugins((
                    lightyear::prelude::client::ClientPlugins { tick_duration },
                    ClientRendererPlugin::new(format!("Client {client_id:?}")),
                ));
                app
            }
            #[cfg(feature = "server")]
            Some(Mode::Server) => {
                app.add_plugins((lightyear::prelude::server::ServerPlugins { tick_duration },));
                app.set_error_handler(bevy::ecs::error::ignore);
                app
            }
            None => {
                panic!("Mode is required");
            }
            _ => {
                todo!()
            }
        }
    }

    pub fn spawn_connections(&self, app: &mut App) {
        let conditioner = LinkConditionerConfig::average_condition();
        match self.mode {
            #[cfg(feature = "client")]
            Some(Mode::Client { client_id }) => {
                let client = app
                    .world_mut()
                    .spawn(PlayerClient {
                        client_id: client_id.expect("You need to specify a client_id via `-c ID`"),
                        client_port: CLIENT_PORT,
                        server_addr: SERVER_ADDR,
                        conditioner: Some(RecvLinkConditioner::new(conditioner.clone())),
                        transport: ClientTransports::WebTransport,
                        shared: SHARED_SETTINGS,
                    })
                    .id();
                app.add_systems(Startup, connect);
            }
            #[cfg(feature = "server")]
            Some(Mode::Server) => {
                let server = app
                    .world_mut()
                    .spawn(CoreServer {
                        conditioner: None,
                        transport: ServerTransports::Udp {
                            local_port: SERVER_PORT,
                        },
                        shared: SHARED_SETTINGS,
                    })
                    .id();
                app.add_systems(Startup, start);
            }
            _ => {}
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    #[cfg(feature = "client")]
    /// Runs the app in client mode
    Client {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
    /// Runs the app in server mode
    #[cfg(feature = "server")]
    Server,
}

impl Default for Mode {
    fn default() -> Self {
        #[cfg(feature = "client")]
        return Mode::Client { client_id: None };
        #[cfg(all(feature = "server", not(feature = "client")))]
        return Mode::Server;
    }
}

struct SendApp(App);

unsafe impl Send for SendApp {}
impl SendApp {
    fn run(&mut self) {
        self.0.run();
    }
}

impl Default for Cli {
    fn default() -> Self {
        cli()
    }
}

pub fn cli() -> Cli {
    Cli::parse()
}

pub fn client_app() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().set(AssetPlugin {
        // https://github.com/bevyengine/bevy/issues/10157
        meta_check: bevy::asset::AssetMetaCheck::Never,
        ..default()
    }));
    // we want the same frequency of updates for both focused and unfocused
    app
}

pub fn server_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, DiagnosticsPlugin));
    app
}
