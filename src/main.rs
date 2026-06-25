use bevy::{ecs::entity::EntityHashMap, prelude::*};
use enterpolation::{Signal, bspline::BSpline};

pub mod route;
pub mod vehicle;

use route::*;
use vehicle::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_simulation, setup_map))
        .add_systems(Update, (spawn_vehicles, vehicle_movement, draw_routes, draw_vehicles))
        .run();
}

fn setup_simulation(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn setup_map(mut commands: Commands) {
    let endpoint_id = commands.spawn((EndPoint, Name::new("North Exit"))).id();

    let segment_id = commands
        .spawn(Segment {
            // evaluator: Box::new(|time| Vec3::new(-20.0 + (time * 40.0), 0.0, 0.0)),
            evaluator: Box::new(|time| Vec3::new(0.0, -20.0 + (time * 40.0), 0.0)),
            length: 40.0,
            speed_limit: 13.9, // ~50kmh-1
        })
        .id();

    commands.spawn(Connection {
        next_segments: vec![endpoint_id],
        requires_yield: false,
    });

    let mut weights = EntityHashMap::default();
    weights.insert(endpoint_id, 100);

    commands.spawn(SpawnPoint {
        vehicles_per_second: 0.5,
        destination_weights: weights,
    });
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
                    commands.entity(entity).despawn();
                }
            } else {
                // += instead?
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
        gizmos.linestrip(points, Color::hsl(120.0, 1.0, 0.5));
    }
}

fn draw_vehicles(
    mut gizmos: Gizmos,
    vehicles: Query<&Transform, With<Navigator>>,
) {
    for transform in vehicles.iter() {
        // Draws a bright cyan circle with a 10.0 pixel/unit radius
        // at the vehicle's current coordinates
        gizmos.circle_2d(transform.translation.truncate(), 10.0, Color::linear_rgb(0.0, 150.0, 250.0));
    }
}
