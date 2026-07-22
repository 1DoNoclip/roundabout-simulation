use crate::*;

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Kinematics>()
            .register_type::<Navigator>();
    }
}

#[derive(Component, Reflect)]
/// The motion characteristics for the vehicle.
pub struct Kinematics {
    pub speed: Speed,
    /// Target speed that the driver would aim for on an empty road.
    pub target_speed: Speed,
    pub max_acceleration: f32,
    pub max_deceleration: f32,
}

#[derive(Component, Reflect)]
/// Decides how the vehicle navigates the map.
pub struct Navigator {
    /// The route for the vehicle to follow.
    pub route: Vec<Entity>,
    /// An index of route to identify the current segment.
    pub current_segment: usize,
    /// A segment progress between 0 and 1.
    pub progress: f32,
}

pub fn spawn_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    spawn_points: Query<&SpawnPoint>,
    segments: Query<&Segment>,
) {
    let delta_time = time.delta_secs();

    for spawn_point in spawn_points {
        // Note: Replace spawning probability with Poisson Process.
        // The current implementation has an issue where if there is a lag spike,
        // the spawn probability will exceed 100%, however only 1 vehicle is spawned.
        // This means the extra value above 100% is lost, resulting in incorrect spawn rates.
        // Poisson Process uses an exponential curve, where the average spawn rate = max_vehicles_per_second
        // (assuming that the road has capacity to spawn vehicles), but with the advantage of variance
        // of spawn rates.
        let frame_probability = spawn_point.max_vehicles_per_second * delta_time;
        if rand::random::<f32>() < frame_probability {
            // Pathfinding.
            let segment1_id = spawn_point.segment;
            let Ok(segment1) = segments.get(segment1_id) else {
                continue;
            };

            let segment2_id = match &segment1.connection {
                Connection::NextSegments { next_segments, .. } => next_segments
                    .first()
                    .expect("expected Segment 2 at index 0"),
                Connection::EndPoint { .. } => continue,
            };

            let initial_route = vec![segment1_id, *segment2_id];
            let start_position = (segment1.evaluator)(0.0);

            // Spawning.
            commands.spawn((
                Name::new("Vehicle"),
                Kinematics {
                    speed: Speed::from_miles_per_hour(5.0).expect("failed to create Speed"),
                    target_speed: Speed::from_miles_per_hour(60.0).expect("failed to create Speed"),
                    max_acceleration: 3.0,
                    max_deceleration: 8.0,
                },
                Navigator {
                    route: initial_route,
                    current_segment: 0,
                    progress: 0.0,
                },
                // make visible
                Transform::from_translation(start_position),
                Visibility::default(),
            ));
        }
    }
}

pub fn vehicle_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    vehicles: Query<(Entity, &mut Kinematics, &mut Navigator, &mut Transform)>,
) {
    let delta_seconds = time.delta_secs();

    for (entity, mut kinematics, mut navigator, mut transform) in vehicles {
        if navigator.current_segment >= navigator.route.len() {
            continue;
        }

        let segment_id = navigator.route[navigator.current_segment];

        if let Ok(segment) = segments.get(segment_id) {
            let delta_progress = (*kinematics.speed * delta_seconds) / segment.length;
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
                transform.translation = segment.sample_clamped(navigator.progress);
            }

            // Increases speed due to acceleration.
            if *kinematics.speed < *kinematics.target_speed {
                *kinematics.speed += kinematics.max_acceleration * delta_seconds;
                if *kinematics.speed > *kinematics.target_speed {
                    *kinematics.speed = *kinematics.target_speed;
                }
            }
        }
    }
}
