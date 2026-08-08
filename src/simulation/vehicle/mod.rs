use crate::{simulation::select_destination_arm, *};
use rand::{RngExt, SeedableRng, rng, rngs::StdRng};

pub(crate) mod components;

pub(crate) use components::*;

pub(super) struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Kinematics>()
            .register_type::<Navigator>();
    }
}

#[derive(Bundle)]
struct VehicleBundle {
    name: Name,
    kinematics: Kinematics,
    navigator: Navigator,
    transform: Transform,
}

impl VehicleBundle {
    fn try_new(
        segments: &Query<&Segment>,
        speed: Speed,
        target_speed: Speed,
        max_acceleration: Acceleration,
        max_deceleration: Acceleration,
        route: Vec<Entity>,
    ) -> Result<Self, &'static str> {
        let navigator = Navigator::try_new(route)?;
        let start_segment = segments
            .get(navigator.current_segment_id())
            .expect("expected to find a Segment component");
        Ok(VehicleBundle {
            name: Name::new("Vehicle"),
            kinematics: Kinematics::new(speed, target_speed, max_acceleration, max_deceleration),
            navigator,
            transform: Transform::from_translation(start_segment.sample_clamped(0.0)),
        })
    }
}

/// Used in spawn_vehicles.
#[derive(Deref, DerefMut)]
pub(crate) struct SpawnerRng(StdRng);

// Local requires Default to initialize the struct.
impl Default for SpawnerRng {
    fn default() -> Self {
        Self(StdRng::from_rng(&mut rng()))
    }
}

pub(crate) fn spawn_vehicles(
    mut commands: Commands,
    // A unique instance of SpawnerRng is created for this system (Local).
    // It is handed to us each time Bevy calls this system.
    mut spawner_rng: Local<SpawnerRng>,
    time: Res<Time>,
    roundabout_blueprint: Res<RoundaboutBlueprint>,
    arms: Query<(Entity, &Arm)>,
    spawn_points: Query<(Entity, &SpawnPoint)>,
    end_points: Query<(Entity, &EndPoint)>,
    segments: Query<&Segment>,
) {
    let delta_seconds = time.delta_secs();

    for (spawn_arm_id, spawn_arm) in arms {
        // Temporary: Replace spawning probability with Poisson Process.
        // The current implementation has an issue where if there is a lag spike,
        // the spawn probability will exceed 100%, however only 1 vehicle is spawned.
        // This means the extra value above 100% is lost, resulting in incorrect spawn rates.
        // Poisson Process uses an exponential curve, where the average spawn rate = max_vehicles_per_second
        // (assuming that the road has capacity to spawn vehicles), but with the advantage of variance
        // of spawn rates.
        let frame_probability = spawn_arm.max_vehicles_per_second() * delta_seconds;
        if frame_probability > spawner_rng.random::<f32>() {
            let end_arm_id =
                select_destination_arm(&mut spawner_rng, spawn_arm.destination_weights());
            let (_, end_arm) = arms
                .get(end_arm_id)
                .expect("expected to find an Arm entity with the matching Arm entity");

            // In future, this will be gotten from the zone system.
            let lane_index = select_lane_index(
                spawn_arm,
                end_arm,
                roundabout_blueprint.arm_blueprints().len(),
                roundabout_blueprint.number_of_lanes(),
            );

            let spawn_point = spawn_points
                .iter()
                .find(|(_, spawn_point)| {
                    spawn_point.arm() == spawn_arm_id && spawn_point.lane_index() == lane_index
                })
                .map(|(_, spawn_point)| spawn_point)
                .expect("expected to find one SpawnPoint entity with matching lane_index");

            // Pathfinding.
            let route =
                calculate_route(&arms, &end_points, &segments, spawn_point, end_arm.index())
                    .expect("failed to pathfind from SpawnPoint to EndPoint");

            commands.spawn(
                VehicleBundle::try_new(
                    &segments,
                    Speed::from_miles_per_hour(5.0).expect("failed to create"),
                    Speed::from_miles_per_hour(60.0).expect("failed to create"),
                    Acceleration::new(3.0),
                    Acceleration::new(-8.0),
                    route,
                )
                .expect("failed to spawn VehicleBundle"),
            );
        }
    }
}

pub(super) fn move_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    vehicles: Query<(Entity, &mut Kinematics, &mut Navigator, &mut Transform)>,
) {
    let delta_seconds = time.delta_secs();

    for (entity, mut kinematics, mut navigator, mut transform) in vehicles {
        let segment_id = navigator.current_segment_id();

        if let Ok(segment) = segments.get(segment_id) {
            let delta_progress = (*kinematics.speed * delta_seconds) / segment.length();
            match navigator.add_progress(delta_progress) {
                Ok(_) => transform.translation = segment.sample_clamped(navigator.progress()),
                Err(_) => {
                    match navigator.increment_current_segment_index() {
                        Ok(_) => navigator.reset_progress(),
                        Err(_) => {
                            // Reached the end point (add stats in future)
                            statistics.increment_total_vehicles_passed();
                            commands.entity(entity).despawn();
                        }
                    }
                }
            }

            // Increases speed due to acceleration.
            if *kinematics.speed < *kinematics.target_speed() {
                *kinematics.speed = (*kinematics.speed
                    + *kinematics.max_acceleration() * delta_seconds)
                    .min(*kinematics.target_speed());
            }
        }
    }
}

const fn select_lane_index(
    entry_arm: &Arm,
    exit_arm: &Arm,
    number_of_arms: usize,
    number_of_lanes: usize,
) -> usize {
    // Single-lane roundabouts always use lane 0.
    if number_of_lanes == 1 {
        return 0;
    }

    let exit_rank = get_exit_rank(entry_arm, exit_arm, number_of_arms);
    // U-turns always use innermost lane.
    if exit_rank == 0 {
        return 0;
    }

    let max_rank = (number_of_arms - 1) as f32;
    let rank_step = (exit_rank - 1) as f32;
    let total_lanes = (number_of_lanes - 1) as f32;
    // Linearly map the ratio and round to the nearest lane index
    let outer_to_inner_ratio = (rank_step * total_lanes / max_rank).round() as usize;

    (number_of_lanes - 1) - outer_to_inner_ratio
}

/// Returns a 1-based exit rank for a vehicle travelling from `entry_arm` to `exit_arm`.
const fn get_exit_rank(entry_arm: &Arm, exit_arm: &Arm, number_of_arms: usize) -> usize {
    (exit_arm.index() + number_of_arms - entry_arm.index()) % number_of_arms
}

// fn select_lane_index(
//     roundabout_blueprint: &Res<RoundaboutBlueprint>,
//     arms: &Query<(Entity, &Arm)>,
//     spawn_arm: &Arm,
//     end_arm: &Arm,
// ) -> usize {
//     let angle_difference = clockwise_angle_difference(spawn_arm.angle(), end_arm.angle());
//     info!("{angle_difference}");
//     0
// }

// /// Returns the clockwise angle difference from `from` to `to` in radians.
// fn clockwise_angle_difference(from: Rot2, to: Rot2) -> f32 {
//     (from.as_radians() - to.as_radians()).rem_euclid(std::f32::consts::TAU)
// }

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
