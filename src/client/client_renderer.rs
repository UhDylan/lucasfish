use bevy::picking::prelude::{Click, Pointer};
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

pub struct ClientRendererPlugin {
    pub name: String,
}

impl ClientRendererPlugin {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Resource)]
struct GameName(String);

impl Plugin for ClientRendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameName(self.name.clone()));
        app.insert_resource(ClearColor::default());
        app.add_systems(Startup, set_window_title);
    }
}

fn set_window_title(mut window: Query<&mut Window>, game_name: Res<GameName>) {
    let mut window = window.single_mut().unwrap();
    window.title = format!("HELP.{}", game_name.0);
}
