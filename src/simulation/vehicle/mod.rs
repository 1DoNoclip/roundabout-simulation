use crate::*;
use rand::{RngExt, SeedableRng, distr::{Distribution, weighted::WeightedIndex}, rng, rngs::StdRng, seq::IteratorRandom};

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Kinematics>()
            .register_type::<Navigator>();
    }
}

/// The motion characteristics for the vehicle.
#[derive(Component, Reflect)]
pub struct Kinematics {
    pub speed: Speed,
    /// Target speed that the driver would aim for on an empty road.
    pub target_speed: Speed,
    pub max_acceleration: f32,
    pub max_deceleration: f32,
}

/// Decides how the vehicle navigates the map.
#[derive(Component, Reflect)]
pub struct Navigator {
    /// The route for the vehicle to follow.
    pub route: Vec<Entity>,
    /// An index of route to identify the current segment.
    pub current_segment: usize,
    /// A segment progress between 0 and 1.
    pub progress: f32,
}

/// Used in spawn_vehicles.
#[derive(Deref, DerefMut)]
pub struct SpawnerRng(StdRng);

// Local requires Default to initialize the struct.
impl Default for SpawnerRng {
    fn default() -> Self {
        Self(StdRng::from_rng(&mut rng()))
    }
}

pub fn spawn_vehicles(
    mut commands: Commands,
    // A unique instance of SpawnerRng is created for this system (Local).
    // It is handed to us each time Bevy calls this system.
    mut spawner_rng: Local<SpawnerRng>,
    time: Res<Time>,
    arms: Query<(Entity, &Arm)>,
    spawn_points: Query<&SpawnPoint>,
    segments: Query<&Segment>,
) {
    let delta_seconds = time.delta_secs();

    for (arm_id, arm) in arms {
        // For now the chosen lane will be randomly selected before pathfinding is implemented.
        let arm_spawn_points = spawn_points
            .iter()
            .filter(|spawn_point| spawn_point.arm == arm_id);

        // Temporary: Replace spawning probability with Poisson Process.
        // The current implementation has an issue where if there is a lag spike,
        // the spawn probability will exceed 100%, however only 1 vehicle is spawned.
        // This means the extra value above 100% is lost, resulting in incorrect spawn rates.
        // Poisson Process uses an exponential curve, where the average spawn rate = max_vehicles_per_second
        // (assuming that the road has capacity to spawn vehicles), but with the advantage of variance
        // of spawn rates.
        let frame_probability = arm.max_vehicles_per_second * delta_seconds;
        if frame_probability > spawner_rng.random::<f32>() {
            // Temporary: In future, the lane will be chosen based on destination.
            // Destination weights will choose a destination.
            // The lane (and therefore, spawn point) will be chosen based on the arm angle
            // difference between the entry and exit arms and the number of lanes.
            let spawn_point = arm_spawn_points
                .choose(&mut spawner_rng)
                .expect("no SpawnPoints found for this Arm");

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
            let Ok(segment2) = segments.get(*segment2_id) else {
                continue;
            };

            let segment3_id = match &segment2.connection {
                Connection::NextSegments { next_segments, .. } => next_segments
                    .first()
                    .expect("expected Segment 3 at index 0"),
                Connection::EndPoint { .. } => continue,
            };
            let Ok(segment3) = segments.get(*segment3_id) else {
                continue;
            };
            let segment4_id = match &segment3.connection {
                Connection::NextSegments { next_segments, .. } => next_segments
                    .first()
                    .expect("expected Segment 4 at index 0"),
                Connection::EndPoint { .. } => continue,
            };
            let Ok(segment4) = segments.get(*segment4_id) else {
                continue;
            };
            let segment5_id = match &segment4.connection {
                Connection::NextSegments { next_segments, .. } => next_segments
                    .first()
                    .expect("expected Segment 5 at index 0"),
                Connection::EndPoint { .. } => continue,
            };

            let initial_route = vec![
                segment1_id,
                *segment2_id,
                *segment3_id,
                *segment4_id,
                *segment5_id,
            ];
            let start_position = (segment1.evaluator)(0.0);

            // Spawning.
            commands.spawn((
                Name::new("Vehicle"),
                Kinematics {
                    speed: Speed::from_miles_per_hour(5.0).expect("failed to create"),
                    target_speed: Speed::from_miles_per_hour(60.0).expect("failed to create"),
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

fn select_destination_arm(mut spawner_rng: &mut SpawnerRng, destination_weights: &DestinationWeights) -> Entity {
    if destination_weights.is_empty() {
        panic!("Cannot select a destination arm from an empty destination_weights");
    }

    let arms = destination_weights.keys().cloned().collect::<Vec<_>>();
    let weights = destination_weights.values().cloned().collect::<Vec<_>>();

    let distribution = WeightedIndex::new(&weights).expect("failed to create WeightedIndex, ensure that not every weight is zero");
    let selected_index = distribution.sample(&mut spawner_rng);
    arms[selected_index]
}
