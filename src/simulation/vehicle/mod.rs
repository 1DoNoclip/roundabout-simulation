use crate::*;
use rand::{SeedableRng, rng, rngs::StdRng};

pub(crate) mod components;
pub(crate) mod kinematics;
mod pathfinding;

pub(crate) use components::*;
pub(crate) use kinematics::*;
use pathfinding::*;
use rand_distr::{Distribution, Poisson};

pub(super) struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ComponentsPlugin, KinematicsPlugin, PathfindingPlugin));
    }
}

#[derive(Bundle)]
struct VehicleBundle {
    name: Name,
    idm_driver: IdmDriver,
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
            .get(
                navigator
                    .current_segment_id()
                    .expect("expected .current_segment_id() to be Some"),
            )
            .expect("expected to find a Segment component");
        Ok(VehicleBundle {
            name: Name::new("Vehicle"),
            idm_driver: IdmDriver::default(),
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

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_vehicles(
    mut commands: Commands,
    // A unique instance of SpawnerRng is created for this system (Local).
    // It is handed to us each time Bevy calls this system.
    mut spawner_rng: Local<SpawnerRng>,
    time: Res<Time>,
    roundabout_blueprint: Res<RoundaboutBlueprint>,
    mut spawning_arms: Query<(Entity, &Arm, &mut VehicleSpawnQueue)>,
    arms: Query<&Arm>,
    target_arms: Query<&Arm>,
    spawn_points: Query<(Entity, &SpawnPoint)>,
    end_points: Query<(Entity, &EndPoint)>,
    segments: Query<&Segment>,
    existing_vehicles: Query<(&Navigator, &Transform), With<Kinematics>>,
) {
    let delta_seconds = time.delta_secs();

    for (spawn_arm_id, spawn_arm, mut spawn_queue) in &mut spawning_arms {
        // Poisson Process uses an exponential curve, where the average spawn rate = max_vehicles_per_second
        // (assuming that the road has capacity to spawn vehicles), but with the advantage of variance
        // of spawn rates.

        // Calculate expected number of new vehicles during this frame.
        let lambda = spawn_arm.max_vehicles_per_second() * delta_seconds;
        if lambda > 0.0
            && let Ok(poisson) = Poisson::new(lambda)
        {
            let number_to_spawn = poisson.sample(&mut spawner_rng) as u32;
            spawn_queue.reserve(number_to_spawn as usize);
            for _ in 0..number_to_spawn {
                let end_arm_id =
                    select_destination_arm(&mut spawner_rng, spawn_arm.destination_weights());
                spawn_queue.push_back(end_arm_id);
            }
        }

        let mut drained_indices = Vec::new();
        let mut frame_spawned_segments = Vec::new();

        for (index, &end_arm_id) in spawn_queue.iter().enumerate() {
            let end_arm = target_arms
                .get(end_arm_id)
                .expect("expected to find a matching target Arm entity");

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
                .expect("expected to find a matching SpawnPoint with lane_index");

            let entry_segment_id = spawn_point.segment();
            let entry_segment = segments
                .get(entry_segment_id)
                .expect("expected Segment component for this Entity");

            let is_blocked_existing = existing_vehicles.iter().any(|(navigator, transform)| {
                navigator.current_segment_id() == Some(entry_segment_id)
                    && transform
                        .translation
                        .distance_squared(entry_segment.start_position())
                        < 25.0
            });
            let is_blocked_this_frame = frame_spawned_segments.contains(&entry_segment_id);
            if is_blocked_existing || is_blocked_this_frame {
                // We cannot spawn another vehicle in this lane at this moment.
                // Continue to allow vehicles in other unblocked lanes to spawn.
                continue;
            }

            spawn_vehicle(
                &mut commands,
                &arms,
                &segments,
                spawn_point,
                &end_points,
                end_arm,
            );

            drained_indices.push(index);
            frame_spawned_segments.push(entry_segment_id);
        }

        for index in drained_indices.into_iter().rev() {
            spawn_queue.remove(index);
        }
    }
}

pub(super) fn move_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    mut vehicle_params: ParamSet<(
        Query<(Entity, &Kinematics, &Navigator)>,
        Query<(Entity, &IdmDriver, &Kinematics, &Navigator)>,
        Query<(Entity, &mut Kinematics, &mut Navigator, &mut Transform)>,
    )>,
) {
    let delta_seconds = time.delta_secs();

    let mut accelerations = Vec::new();

    // Collect driver and kinematic values to release borrow on vehicle_params (so it can be used in the loop).
    let vehicle_drivers = vehicle_params
        .p1()
        .iter()
        .map(|(id, idm_driver, kinematics, _)| {
            (
                id,
                *kinematics.speed,
                *kinematics.target_speed(),
                *kinematics.max_acceleration(),
                idm_driver.exponent(),
                idm_driver.time_headway().as_secs_f32(),
                *idm_driver.comfortable_acceleration(),
            )
        })
        .collect::<Vec<_>>();

    for (
        id,
        current_speed,
        target_speed,
        max_acceleration,
        exponent,
        time_headway_seconds,
        comfortable_acceleration,
    ) in vehicle_drivers
    {
        // Free road acceleration term.
        let free_road_acceleration =
            max_acceleration * (1.0 - (current_speed / target_speed)).powf(exponent);

        let interaction_acceleration =
            if let Ok(lead_vehicle_info) = find_lead_vehicle(&segments, &vehicle_params.p0(), id) {
                let delta_v = current_speed - *lead_vehicle_info.speed;

                let dynamic_gap = (current_speed * delta_v)
                    / (2.0 * (max_acceleration * comfortable_acceleration).sqrt());
                let s_star = IdmDriver::MIN_STATIONARY_DISTANCE
                    + (current_speed * time_headway_seconds)
                    + dynamic_gap.max(0.0);

                let gap = *lead_vehicle_info.distance;
                -max_acceleration * (s_star / gap).powi(2)
            } else {
                0.0
            };

        let total_acceleration =
            Acceleration::new(free_road_acceleration + interaction_acceleration);
        accelerations.push((id, total_acceleration));
    }

    for (id, acceleration) in accelerations {
        if let Ok((_, mut kinematics, mut navigator, mut transform)) =
            vehicle_params.p2().get_mut(id)
        {
            *kinematics.speed = (*kinematics.speed + *acceleration * delta_seconds).max(0.0);

            let current_segment_id = navigator
                .current_segment_id()
                .expect("expected .current_segment_id() to be Some");

            if let Ok(segment) = segments.get(current_segment_id) {
                let delta_progress = (*kinematics.speed * delta_seconds) / segment.length();
                match navigator.add_progress(delta_progress) {
                    Ok(_) => transform.translation = segment.sample_clamped(navigator.progress()),
                    Err(overflow_progress) => match navigator.increment_current_segment_index() {
                        Ok(_) => {
                            navigator.reset_progress();
                            if let Some(next_segment_id) = navigator.current_segment_id() {
                                if let Ok(next_segment) = segments.get(next_segment_id) {
                                    transform.translation =
                                        next_segment.sample_clamped(navigator.progress());
                                }
                            }
                        }
                        Err(_) => {
                            statistics.increment_total_vehicles_passed();
                            commands.entity(id).despawn();
                        }
                    },
                }
            }
        }
    }
}

fn spawn_vehicle(
    commands: &mut Commands,
    arms: &Query<&Arm>,
    segments: &Query<&Segment>,
    spawn_point: &SpawnPoint,
    end_points: &Query<(Entity, &EndPoint)>,
    end_arm: &Arm,
) {
    let route = calculate_route(arms, end_points, segments, spawn_point, end_arm.index())
        .expect("failed to pathfind from SpawnPoint to EndPoint");

    commands.spawn(
        VehicleBundle::try_new(
            segments,
            Speed::from_miles_per_hour(5.0).expect("failed to create"),
            Speed::from_miles_per_hour(60.0).expect("failed to create"),
            Acceleration::new(3.0),
            Acceleration::new(-8.0),
            route,
        )
        .expect("failed to spawn VehicleBundle"),
    );
}

/// Finds the vehicle in front of this vehicle.
///
/// Returns `None` if a lead vehicle was not found.
/// ### Arguments
/// * `this_vehicle_id` - The vehicle to find the lead vehicle for.
fn find_lead_vehicle(
    segments: &Query<&Segment>,
    vehicles: &Query<(Entity, &Kinematics, &Navigator)>,
    this_vehicle_id: Entity,
) -> Result<LeadVehicleInfo, String> {
    let (_, _, this_navigator) = vehicles
        .get(this_vehicle_id)
        .map_err(|error| error.to_string())?;

    let this_route = this_navigator.route();
    let Some(this_current_segment_id) = this_navigator.current_segment_id() else {
        return Err("this vehicle has reached the end of its route".to_owned());
    };
    let this_progress = this_navigator.progress();

    // The route from the this's current segment to the end.
    let existing_route = if let Some(current_segment_index) = this_route
        .iter()
        .position(|&segment_id| segment_id == this_current_segment_id)
    {
        // From current segment to end of route.
        // We ignore segments that this vehicle has already travelled as we are looking ahead.
        &this_route[current_segment_index..]
    } else {
        return Err(format!(
            "failed to find this_current_segment_id ({this_current_segment_id}) in this_route ({this_route:?})"
        ));
    };

    // (route_index, progress, vehicle_entity_id)
    let mut best_lead_vehicle: Option<(usize, f32, Entity)> = None;
    for (vehicle_id, _, navigator) in vehicles {
        if vehicle_id == this_vehicle_id {
            continue;
        }

        // If the vehicle is still on the map.
        if let Some(current_segment_id) = navigator.current_segment_id() {
            // If the vehicle is on the route of `this_vehicle`.
            if let Some(index) = existing_route
                .iter()
                .position(|&segment_id| current_segment_id == segment_id)
            {
                let progress = navigator.progress();

                // If they are on the same segment, but the vehicle's progress is less than
                // `this_vehicle`'s progress, then skip as we are only looking ahead.
                if index == 0 && progress <= this_progress {
                    continue;
                }

                // Tuples implement comparison (compares .0 first then .1 after).
                let candidate_rank = (index, progress);

                if let Some((best_index, best_progress, _)) = best_lead_vehicle {
                    // If this candidate is better than the current best, then replace it.
                    if candidate_rank < (best_index, best_progress) {
                        best_lead_vehicle = Some((index, progress, vehicle_id));
                    }
                } else {
                    // If there is no current best, then this vehicle must (currently) be the best.
                    best_lead_vehicle = Some((index, progress, vehicle_id));
                }
            }
        }
    }

    let (lead_index, lead_progress, lead_vehicle_id) =
        best_lead_vehicle.ok_or("failed to find a lead vehicle")?;

    let total_distance = Distance::try_new(if lead_index == 0 {
        let segment_length = segments
            .get(existing_route[0])
            .map_err(|_| "expected to get segment component")?
            .length();
        (lead_progress - this_progress) * segment_length
    } else {
        let mut total_distance = 0.0;

        let first_segment_length = segments
            .get(existing_route[0])
            .map_err(|_| "expected to get segment component")?
            .length();
        total_distance += (1.0 - this_progress) * first_segment_length;

        for index in 1..lead_index {
            let segment_length = segments
                .get(existing_route[index])
                .map_err(|_| "expected to get segment component")?
                .length();
            total_distance += segment_length;
        }

        let last_segment_length = segments
            .get(existing_route[lead_index])
            .map_err(|_| "expected to get segment component")?
            .length();
        total_distance += lead_progress * last_segment_length;

        total_distance
    })?;

    let (_, lead_kinematics, _) = vehicles
        .get(lead_vehicle_id)
        .expect("expected to find vehicle components");

    Ok(LeadVehicleInfo {
        distance: total_distance,
        speed: lead_kinematics.speed,
    })
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
