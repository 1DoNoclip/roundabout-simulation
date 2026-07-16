use bevy::{ecs::entity::EntityHashMap, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use enterpolation::{Signal, linear::Linear};

pub mod blueprint;
pub mod curve;
pub mod layout;
pub mod route;
pub mod statistics;
pub mod vehicle;

use blueprint::*;
use curve::*;
use layout::*;
use route::*;
use statistics::*;
use vehicle::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::default(),
            RoutePlugin,
            StatisticsPlugin,
            VehiclePlugin,
        ))
        .add_systems(Startup, (setup_simulation, setup_map))
        .add_systems(
            Update,
            (
                spawn_vehicles,
                vehicle_movement,
                (draw_routes, draw_vehicles).chain(),
            ),
        )
        .run();
}

fn setup_simulation(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(Statistics::default());
}

fn setup_map(mut commands: Commands) {
    let endpoint_id = commands.spawn((Name::new("North Exit"), EndPoint)).id();

    let line = Linear::builder()
        .elements([Vec3::new(0.0, -20.0, 0.0), Vec3::new(0.0, 20.0, 0.0)])
        .equidistant::<f32>()
        .normalized()
        .build()
        .unwrap();

    commands.spawn(Segment::new(
        line,
        SpeedLimit::new(13.9).unwrap(), // ~50kmh-1
    ));

    commands.spawn((
        Name::new("Connection"),
        Connection {
            next_segments: vec![endpoint_id],
            requires_yield: false,
        },
    ));

    let mut weights = EntityHashMap::default();
    weights.insert(endpoint_id, 100);

    commands.spawn((
        Name::new("SpawnPoint"),
        SpawnPoint {
            max_vehicles_per_second: 0.5,
            destination_weights: weights,
        },
    ));
}
