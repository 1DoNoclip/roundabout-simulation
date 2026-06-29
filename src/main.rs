use bevy::{ecs::entity::EntityHashMap, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use enterpolation::{Signal, linear::Linear};

pub mod route;
pub mod statistics;
pub mod vehicle;

use route::*;
use statistics::*;
use vehicle::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::default(),
        ))
        .register_type::<Connection>()
        .register_type::<SpawnPoint>()
        .register_type::<EndPoint>()
        .register_type::<Statistics>()
        .register_type::<Kinematics>()
        .register_type::<Navigator>()
        .add_systems(Startup, (setup_simulation, setup_map))
        .add_systems(
            Update,
            (spawn_vehicles, vehicle_movement, draw_routes, draw_vehicles),
        )
        .run();
}

fn setup_simulation(mut commands: Commands) {
    commands.spawn(Camera2d::default());
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

    commands.spawn(Segment {
        evaluator: Box::new(move |time| line.eval(time)),
        length: 40.0,
        speed_limit: 13.9,  // ~50kmh-1
    });

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
            vehicles_per_second: 0.5,
            destination_weights: weights,
        },
    ));
}

fn spawn_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    spawn_points: Query<&SpawnPoint>,
    segments: Query<Entity, With<Segment>>,
) {
    let delta_time = time.delta_secs();

    for spawn_point in spawn_points {
        let frame_probability = spawn_point.vehicles_per_second * delta_time;
        if rand::random::<f32>() < frame_probability {
            let initial_route = vec![segments.single().unwrap()];

            commands.spawn((
                Name::new("Vehicle"),
                Kinematics {
                    speed: 10.0,
                    target_speed: 13.9,
                    acceleration: 0.0,
                },
                Navigator {
                    route: initial_route,
                    current_segment: 0,
                    progress: 0.0,
                },
                // make visible
                Transform::default(),
                Visibility::default(),
            ));
        }
    }
}

fn vehicle_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    vehicles: Query<(Entity, &Kinematics, &mut Navigator, &mut Transform)>,
) {
    let delta_time = time.delta_secs();

    for (entity, kinematics, mut navigator, mut transform) in vehicles {
        if navigator.current_segment >= navigator.route.len() {
            continue;
        }

        let segment_id = navigator.route[navigator.current_segment];

        if let Ok(segment) = segments.get(segment_id) {
            let delta_progress = (kinematics.speed * delta_time) / segment.length;
            navigator.progress += delta_progress;

            if navigator.progress >= 1.0 {
                if navigator.current_segment + 1 < navigator.route.len() {
                    navigator.current_segment += 1;
                    navigator.progress = 0.0;
                } else {
                    // Reached the end point (add stats in future)
                    statistics.total_vehicles_passed += 1;
                    commands.entity(entity).despawn();
                }
            } else {
                transform.translation = (segment.evaluator)(navigator.progress);
            }
        }
    }
}

fn draw_routes(mut gizmos: Gizmos, query: Query<&Segment>) {
    let resolution = 100;
    for segment in &query {
        let points = (0..=resolution)
            .map(|i| {
                let time = i as f32 / resolution as f32;
                (segment.evaluator)(time)
            })
            .collect::<Vec<_>>();
        gizmos.linestrip(points, Color::hsl(0.0, 0.0, 1.0));
    }
}

fn draw_vehicles(mut gizmos: Gizmos, vehicles: Query<&Transform, With<Navigator>>) {
    for transform in vehicles.iter() {
        // Draws a bright cyan circle with a 10.0 pixel/unit radius
        // at the vehicle's current coordinates
        gizmos.circle_2d(
            transform.translation.truncate(),
            1.0,
            Color::linear_rgb(255.0, 0.0, 0.0),
        );
    }
}
