use roundabout_simulation::*;

fn main() {
    App::new()
        // Core Bevy & third-party plugins.
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::default(),
        ))
        // Domain plugins.
        .add_plugins((
            AppSetupPlugin,
            BlueprintPlugin,
            GraphicsPlugin,
            LayoutPlugin,
            SimulationPlugin,
        ))
        .run();
}
