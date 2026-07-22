use bevy::{ecs::entity::EntityHashMap, math::cubic_splines::LinearSpline, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

pub mod blueprint;
pub mod graphics;
pub mod layout;
pub mod simulation;

use blueprint::*;
use graphics::*;
use layout::*;
use simulation::*;

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
        .add_systems(Startup, (setup_world, setup_layout))
        .run();
}

fn setup_world(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(Statistics::default());
}

fn setup_layout(mut commands: Commands) {
    let end_point_id = commands.spawn((Name::new("EndPoint"), EndPoint)).id();

    let segment2_id = commands
        .spawn(Segment::to_end(
            LinearSpline::new([Vec3::new(100.0, 100.0, 0.0), Vec3::new(100.0, 200.0, 0.0)]),
            end_point_id,
            Speed::from_miles_per_hour(30.0).expect("failed to create SpeedLimit"),
        ))
        .id();

    let segment1_curve_points = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(55.228, 0.0, 0.0),
        Vec3::new(100.0, 44.772, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
    ];
    let line = CubicBezier::new([segment1_curve_points]);
    let segment1_id = commands
        .spawn(Segment::new(
            line,
            Connection::NextSegments {
                next_segments: vec![segment2_id],
                requires_yield: false,
            },
            Speed::from_miles_per_hour(30.0).expect("failed to create SpeedLimit"),
        ))
        .id();

    let weights = EntityHashMap::from_iter([(end_point_id, 100)]);
    commands.spawn((
        Name::new("SpawnPoint"),
        SpawnPoint {
            segment: segment1_id,
            max_vehicles_per_second: 0.5,
            destination_weights: weights,
        },
    ));
}
