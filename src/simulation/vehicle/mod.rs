use crate::*;
use rand::{
    RngExt, SeedableRng,
    distr::{Distribution, weighted::WeightedIndex},
    rng,
    rngs::StdRng,
    seq::IteratorRandom,
};

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

fn select_destination_arm(
    mut spawner_rng: &mut StdRng,
    destination_weights: &DestinationWeights,
) -> Entity {
    if destination_weights.is_empty() {
        panic!("Cannot select a destination arm from an empty destination_weights");
    }

    let arms = destination_weights.keys().cloned().collect::<Vec<_>>();
    let weights = destination_weights.values().cloned().collect::<Vec<_>>();

    let distribution = WeightedIndex::new(&weights)
        .expect("failed to create WeightedIndex, ensure that not every weight is zero");
    let selected_index = distribution.sample(&mut spawner_rng);
    arms[selected_index]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates a fake Bevy entity for testing.
    fn make_test_entity(id: u32) -> Entity {
        Entity::from_raw_u32(id).expect("failed to create Entity from an ID")
    }

    #[test]
    #[should_panic(expected = "Cannot select a destination arm from an empty destination_weights")]
    fn empty_weights_panics() {
        let mut rng = StdRng::seed_from_u64(42);
        let destination_weights = DestinationWeights::default();

        select_destination_arm(&mut rng, &destination_weights);
    }

    #[test]
    fn single_choice_guaranteed() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut destination_weights = DestinationWeights::default();

        let target_arm = make_test_entity(1);
        // Anything above a value of 0 is included in the distribution.
        destination_weights.insert(target_arm, 1);

        // With only one option, it must return that option 100% of the time.
        for _ in 0..100 {
            let selected = select_destination_arm(&mut rng, &destination_weights);
            assert_eq!(selected, target_arm);
        }
    }

    #[test]
    fn zero_weight_ignored() {
        let mut rng = StdRng::seed_from_u64(12345);
        let mut destination_weights = DestinationWeights::default();

        let lucky_arm = make_test_entity(1);
        let unlucky_arm = make_test_entity(2);

        destination_weights.insert(lucky_arm, 10);
        destination_weights.insert(unlucky_arm, 0); // 0% chance of selection.

        // The 0-weight option should never be picked.
        for _ in 0..100 {
            let selected = select_destination_arm(&mut rng, &destination_weights);
            assert_eq!(selected, lucky_arm);
            assert_ne!(selected, unlucky_arm);
        }
    }

    #[test]
    fn statistical_distribution() {
        // Use a fixed seed so the test outcome is completely deterministic.
        // The seed number has no value. Using a fixed seed just ensures that
        // the test never fails due to the seed. The Law of Large Numbers and
        // the tolerance used in the assertions ensures that this test should
        // pass for most seeds.
        let mut rng = StdRng::seed_from_u64(987654321);
        let mut weights = DestinationWeights::default();

        let arm_a = make_test_entity(1);
        let arm_b = make_test_entity(2);

        weights.insert(arm_a, 25); // Should get ~25% of rolls.
        weights.insert(arm_b, 75); // Should get ~75% of rolls.

        let mut count_a = 0;
        let mut count_b = 0;
        let iterations = 10_000;

        for _ in 0..iterations {
            let selected = select_destination_arm(&mut rng, &weights);
            if selected == arm_a {
                count_a += 1;
            } else if selected == arm_b {
                count_b += 1;
            }
        }

        const PERCENTAGE_TOLERANCE: f64 = 2.0;
        // Calculate actual percentages.
        let percentage_a = (count_a as f64 / iterations as f64) * 100.0;
        let percentage_b = (count_b as f64 / iterations as f64) * 100.0;

        // Allow a small statistical tolerance variance (margin of error) of ±2%.
        assert!(
            (percentage_a - 25.0).abs() < PERCENTAGE_TOLERANCE,
            "Arm A variance too high above {PERCENTAGE_TOLERANCE}%: {}%",
            percentage_a
        );
        assert!(
            (percentage_b - 75.0).abs() < PERCENTAGE_TOLERANCE,
            "Arm B variance too high above {PERCENTAGE_TOLERANCE}%: {}%",
            percentage_b
        );
    }
}
