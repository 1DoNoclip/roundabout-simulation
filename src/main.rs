use roundabout_simulation::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::default(),
            BlueprintPlugin,
            GraphicsPlugin,
            LayoutPlugin,
            SimulationPlugin,
        ))
        .add_systems(Startup, (setup_world, setup_roundabout_layout))
        .run();
}

fn setup_world(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(Statistics::default());
}

fn setup_roundabout_layout(mut commands: Commands) {
    // Define the overall intersection blueprint parameters.
    commands.insert_resource(IntersectionBlueprint {
        number_of_lanes: 2,
        deflection_radius: 15.0,
        speed_limit: Speed::from_miles_per_hour(30.0).expect("failed to create Speed"),
        arms: vec![
            // 4-arm roundabout layout.
            ArmBlueprint {
                angle: Rot2::degrees(0.0),
            },
            ArmBlueprint {
                angle: Rot2::degrees(90.0),
            },
            ArmBlueprint {
                angle: Rot2::degrees(180.0),
            },
            ArmBlueprint {
                angle: Rot2::degrees(270.0),
            },
        ],
    });

    // Define the central roundabout dimensions.
    commands.insert_resource(RoundaboutCircleBlueprint { radius: 25.0 });
}
