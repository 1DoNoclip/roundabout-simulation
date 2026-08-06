use roundabout_simulation::*;
use bevy::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

fn main() {
    App::new()
        // Core Bevy & third-party plugins.
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::default(),
        ))
        // Domain plugins.
        .add_plugins(AppSetupPlugin)
        .run();
}
