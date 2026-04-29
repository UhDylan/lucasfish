#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use crate::client::client::PlayerClient;
use crate::client::client::PlayerClientPlugin;
use crate::client::client_renderer;
use crate::client::client_renderer::ClientRendererPlugin;
use crate::libs::cli::*;
use crate::server::server::CoreServerPlugin;
use crate::shared::shared::FIXED_TIMESTEP_HZ;
use crate::shared::shared::SharedPlugin;
use bevy::prelude::*;
use core::time::Duration;
use lightyear::prelude::client::{InputDelayConfig, InputTimelineConfig};
use lightyear::prelude::{Client, InputTimeline, Timeline};
mod client;
mod libs;
mod protocol;
mod server;
mod shared;

fn main() {
    let cli = Cli::default();

    let mut app = cli.build_app(Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ), true);

    app.add_plugins(SharedPlugin);

    cli.spawn_connections(&mut app);

    app.add_plugins(bevy::log::LogPlugin::default());

    match cli.mode {
        #[cfg(feature = "client")]
        Some(Mode::Client { .. }) => {
            app.add_plugins(PlayerClientPlugin);
            add_input_delay(&mut app);
        }
        #[cfg(feature = "server")]
        Some(Mode::Server) => {
            app.add_plugins(CoreServerPlugin);
        }
        _ => {}
    }

    #[cfg(feature = "client")]
    app.run();
    #[cfg(feature = "server")]
    app.run();
}

fn add_input_delay(app: &mut App) {
    let client = app
        .world_mut()
        .query_filtered::<Entity, With<Client>>()
        .single(app.world_mut())
        .unwrap();

    // set some input-delay since we are predicting all entities
    app.world_mut().entity_mut(client).insert(
        InputTimelineConfig::default().with_input_delay(InputDelayConfig::fixed_input_delay(0)),
    );
}
