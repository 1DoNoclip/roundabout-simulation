//! Related to vehicle movement, such as `move_vehicles` system.

use crate::*;
use bimap::BiHashMap;

pub(crate) mod types;

pub(crate) use types::*;

pub(super) struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TypesPlugin);
    }
}

/// Calculates vehicles' accelerations due to the IDM model, road geometry (coming soon), and yield logic.
pub(in crate::simulation) fn calculate_accelerations(
    roundabout_blueprint: Res<RoundaboutBlueprint>,
    conflict_points: Res<RoundaboutConflictPoints>,
    segments: Query<&Segment>,
    entry_line_segments: Query<&Segment, With<segment_type::EntryLine>>,
    exit_deflection_segments: Query<(), With<segment_type::ExitDeflection>>,
    intra_arm_sectors: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors: Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
    vehicles: Query<
        (
            Entity,
            &IdmDriver,
            &Kinematics,
            &Navigator,
            &Speed,
        ),
        With<Vehicle>,
    >,
    circulating_vehicles: Query<(Entity, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
    mut next_accelerations: Query<&mut NextAcceleration, With<Vehicle>>,
    lead_vehicles_query: Query<(Entity, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
) {
    for (id, idm_driver, kinematics, navigator, &speed) in vehicles {
        let mut lead_vehicle_info = find_lead_vehicle(&segments, &lead_vehicles_query, id).ok();

        let current_segment_id = navigator.current_segment_id();
        // If the vehicle is on an entry line, then check if it needs to yield.
        if let Ok(entry_segment) = entry_line_segments.get(current_segment_id) {
            let distance_to_line = Distance::try_new_metres(
                (1.0 - navigator.progress()) * entry_segment.length_metres(),
            )
            .expect("expected to be positive");

            let entry_arm_index = entry_segment.arm_index();
            let entry_lane_index = entry_segment.lane_index();

            if let Ok(circulating_vehicles) = get_circulating_vehicles(
                id,
                entry_lane_index,
                entry_arm_index,
                roundabout_blueprint.number_of_arms(),
                &conflict_points,
                exit_deflection_segments,
                intra_arm_sectors,
                inter_arm_sectors,
                circulating_vehicles,
            ) {
                if should_yield_at_entry(&circulating_vehicles, idm_driver.critical_gap()) {
                    // Virtual object at the yield line, forcing this vehicle to yield.
                    let virtual_lead_vehicle = LeadVehicleInfo {
                        vehicle_kind: VehicleKind::Virtual,
                        distance: distance_to_line,
                        speed: Speed::ZERO,
                    };

                    // Replace the actual lead vehicle with a virtual lead
                    // vehicle if the virtual is closer to the yield line.
                    lead_vehicle_info = match lead_vehicle_info {
                        Some(lead_vehicle_info)
                            if *lead_vehicle_info.distance < *distance_to_line =>
                        {
                            Some(lead_vehicle_info)
                        }
                        _ => Some(virtual_lead_vehicle),
                    }
                }
            }
        }

        let raw_acceleration = idm_driver.calculate_acceleration(
            speed,
            kinematics
                .target_speed()
                .min(roundabout_blueprint.speed_limit()),
            lead_vehicle_info,
        );

        let new_acceleration = Acceleration::new_metres_per_second_squared(raw_acceleration.clamp(
            *kinematics.max_deceleration(),
            *kinematics.max_acceleration(),
        ));

        if let Ok(mut next_acceleration) = next_accelerations.get_mut(id) {
            *next_acceleration = NextAcceleration::from_acceleration(new_acceleration);
        }
    }
}

pub(in crate::simulation) fn update_vehicle_accelerations(
    query: Query<(&mut Acceleration, &NextAcceleration)>,
) {
    for (mut acceleration, &next_acceleration) in query {
        *acceleration = *next_acceleration;
    }
}

pub(in crate::simulation) fn apply_accelerations(
    time: Res<Time>,
    query: Query<(&mut Speed, &Acceleration)>,
) {
    let delta_seconds = time.delta_secs();
    for (mut speed, &acceleration) in query {
        **speed += *acceleration * delta_seconds;
    }
}

/// Moves vehicles along their routes using their accelerations.
///
/// Increments segments once a vehicle has reached the end of the current segment.
pub(in crate::simulation) fn move_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    vehicles: Query<(Entity, &mut Navigator, &mut Transform, &Speed), With<Vehicle>>,
) {
    let delta_seconds = time.delta_secs();
    for (id, mut navigator, mut transform, &speed) in vehicles {
        let current_segment_id = navigator.current_segment_id();
        let Ok(current_segment) = segments.get(current_segment_id) else {
            warn!("Found no segment associated with segment entity.");
            continue;
        };
        let delta_progress = (*speed * delta_seconds) / current_segment.length_metres();
        match navigator.add_progress(delta_progress) {
            Ok(_) => {
                transform.translation = current_segment.sample_clamped(navigator.progress());
            }
            Err(_overflow_progress) => match navigator.increment_current_segment_index() {
                // The vehicle moves onto the next segment.
                Ok(_) => {
                    navigator.reset_progress();
                    // Currently nothing is done with `overflow_progress`, so we do not
                    // need to add any progress to the navigator when on the next segment.
                    // I have not used `overflow_progress` yet as it is often
                    // a very small value so will not have much effect.
                }
                // The vehicle has reached the end and will be despawned.
                Err(_) => {
                    // The vehicle must be despawned to prevent invalid state of Navigator.
                    commands.entity(id).despawn();
                    statistics.increment_total_vehicles_passed();
                }
            },
        }
    }
}

fn get_circulating_vehicles(
    entry_vehicle_id: Entity,
    entry_lane_index: usize,
    entry_arm_index: usize,
    number_of_arms: usize,
    conflict_points: &RoundaboutConflictPoints,
    exit_deflection_segments_query: Query<(), With<segment_type::ExitDeflection>>,
    intra_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
    vehicles: Query<(Entity, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
) -> Result<Vec<CirculatingVehicleInfo>, String> {
    let (intra_arm_sectors, inter_arm_sectors) = get_sectors(
        entry_arm_index,
        number_of_arms,
        intra_arm_sectors_query,
        inter_arm_sectors_query,
    );
    let mut circulating_vehicles = Vec::new();

    for (id, kinematics, navigator, &speed) in vehicles {
        if id == entry_vehicle_id {
            continue;
        }

        let vehicle_length_metres = kinematics.vehicle_length_metres();
        let current_segment_id = navigator.current_segment_id();
        let Some(next_segment_id) = navigator.next_segment_id() else {
            // The vehicle's next segment is not a valid one, therefore
            // the current segment must be the last segment, so is not on
            // a circulating segment.
            continue;
        };
        // If the vehicle is going to exit the circle next, then ignore it.
        // Entering vehicles do not need to yield to exiting vehicles.
        if let Ok(_) = exit_deflection_segments_query.get(next_segment_id) {
            continue;
        }

        // Determine sector type and circulating lane index.
        let (circulating_lane_index, is_inter_arm) =
            match intra_arm_sectors.get_by_left(&current_segment_id) {
                Some(&lane_index) => (lane_index, false),
                None => match inter_arm_sectors.get_by_left(&current_segment_id) {
                    Some(&lane_index) => (lane_index, true),
                    None => continue,
                },
            };

        // Fetch conflict point mapping.
        let (conflict_point_index, _) =
            ConflictPointIndex::try_new(entry_arm_index, entry_lane_index, circulating_lane_index)
                .ok_or_else(|| "failed to create ConflictPointIndex".to_owned())?;

        let Some(conflict_point) = conflict_points.get(conflict_point_index) else {
            continue;
        };

        // Calculate distance to conflict point based on sector type.
        let distance_to_conflict_metres =
            if is_inter_arm {
                // If on the intra arm segment.
                let (_, inter_arm_segment) = inter_arm_sectors_query
                    .get(current_segment_id)
                    .map_err(|_| {
                        format!("expected Segment for inter arm Entity {current_segment_id:?}")
                    })?;

                let intra_arm_sector_id = conflict_point.intra_arm_sector_id;
                let (_, intra_arm_segment) = intra_arm_sectors_query
                    .get(intra_arm_sector_id)
                    .map_err(|_| {
                        format!("expected Segment for intra arm Entity {intra_arm_sector_id:?}")
                    })?;

                intra_arm_segment.length_metres() * conflict_point.intra_arm_sector_progress
                    + inter_arm_segment.length_metres() * (1.0 - navigator.progress())
            // If on the intra arm segment.
            } else {
                let (_, intra_arm_segment) = intra_arm_sectors_query
                    .get(current_segment_id)
                    .map_err(|_| {
                        format!("expected Segment for intra arm Entity {current_segment_id:?}")
                    })?;

                intra_arm_segment.length_metres()
                    * (conflict_point.intra_arm_sector_progress - navigator.progress())
            };

        // Retain in vector if vehicle is approaching or still clearing the conflict zone.
        if distance_to_conflict_metres > -vehicle_length_metres {
            circulating_vehicles.push(CirculatingVehicleInfo {
                distance_to_conflict_metres,
                speed,
                vehicle_length_metres,
            });
        }
    }

    Ok(circulating_vehicles)
}

/// Gets the relevant sectors for vehicles at `arm_index` to yield to.
///
/// ### Arguments
/// * `entry_arm_index` - The arm index of entry vehicles.
/// * `number_of_arms` - Used to calculate the previous arm index for getting inter arm sectors.
///
/// ### Returns
/// `(BiHashMap<Entity, usize>, BiHashMap<Entity, usize>)` - The `usize` is the lane index of that sector.
/// * `.0` is intra arm sectors.
/// * `.1` is inter arm sectors.
fn get_sectors(
    entry_arm_index: usize,
    number_of_arms: usize,
    intra_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
) -> (BiHashMap<Entity, usize>, BiHashMap<Entity, usize>) {
    let intra_arm_sectors =
        intra_arm_sectors_query
            .iter()
            .fold(BiHashMap::new(), |mut map, (id, segment)| {
                if segment.arm_index() == entry_arm_index {
                    map.insert(id, segment.lane_index());
                }
                map
            });

    // The `inter_arm_sectors` are the sector behind the `intra_arm_sectors`.
    let prev_arm_index = (entry_arm_index + number_of_arms - 1) % number_of_arms;
    let inter_arm_sectors =
        inter_arm_sectors_query
            .iter()
            .fold(BiHashMap::new(), |mut map, (id, segment)| {
                if segment.arm_index() == prev_arm_index {
                    map.insert(id, segment.lane_index());
                }
                map
            });

    (intra_arm_sectors, inter_arm_sectors)
}

/// Finds the vehicle in front of this vehicle.
///
/// Returns `None` if a lead vehicle was not found.
/// ### Arguments
/// * `this_vehicle_id` - The vehicle to find the lead vehicle for.
fn find_lead_vehicle(
    segments: &Query<&Segment>,
    vehicles: &Query<(Entity, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
    this_vehicle_id: Entity,
) -> Result<LeadVehicleInfo, String> {
    let (_, _, this_navigator, _) = vehicles
        .get(this_vehicle_id)
        .map_err(|error| error.to_string())?;

    let this_route = this_navigator.route();
    let this_current_segment_id = this_navigator.current_segment_id();
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
    for (vehicle_id, _, navigator, _) in vehicles {
        if vehicle_id == this_vehicle_id {
            continue;
        }

        // If the vehicle is still on the map.
        let current_segment_id = navigator.current_segment_id();
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

    let (lead_index, lead_progress, lead_vehicle_id) =
        best_lead_vehicle.ok_or("failed to find a lead vehicle")?;

    let total_distance = Distance::try_new_metres(if lead_index == 0 {
        let segment_length = segments
            .get(existing_route[0])
            .map_err(|_| "expected to get segment component")?
            .length_metres();
        (lead_progress - this_progress) * segment_length
    } else {
        let mut total_distance = 0.0;

        let first_segment_length = segments
            .get(existing_route[0])
            .map_err(|_| "expected to get segment component")?
            .length_metres();
        total_distance += (1.0 - this_progress) * first_segment_length;

        for index in 1..lead_index {
            let segment_length = segments
                .get(existing_route[index])
                .map_err(|_| "expected to get segment component")?
                .length_metres();
            total_distance += segment_length;
        }

        let last_segment_length = segments
            .get(existing_route[lead_index])
            .map_err(|_| "expected to get segment component")?
            .length_metres();
        total_distance += lead_progress * last_segment_length;

        total_distance
    })?;

    let (_, kinematics, _, lead_speed) = vehicles
        .get(lead_vehicle_id)
        .expect("expected to find vehicle components");

    Ok(LeadVehicleInfo {
        vehicle_kind: VehicleKind::Real {
            length_metres: kinematics.vehicle_length_metres(),
        },
        distance: total_distance,
        speed: *lead_speed,
    })
}

fn should_yield_at_entry(
    circulating_vehicles: &[CirculatingVehicleInfo],
    critical_gap: Duration,
) -> bool {
    let critical_gap_seconds = critical_gap.as_secs_f32();

    for circulating_vehicle in circulating_vehicles {
        // Ignore vehicles that have passed the conflict zone.
        if circulating_vehicle.distance_to_conflict_metres
            < -circulating_vehicle.vehicle_length_metres
        {
            continue;
        }
        // If the vehicle is queued near the conflict zone then prevent entry.
        else if circulating_vehicle.distance_to_conflict_metres < 8.0
            && *circulating_vehicle.speed < 0.5
        {
            return true;
        }

        // Avoid division by zero.
        let speed_metres_per_second = circulating_vehicle.speed.max(0.1);
        let time_to_conflict =
            circulating_vehicle.distance_to_conflict_metres / speed_metres_per_second;

        if time_to_conflict < critical_gap_seconds {
            return true;
        }
    }

    false
}

#[derive(Clone, Copy, Debug)]
struct CirculatingVehicleInfo {
    distance_to_conflict_metres: f32,
    speed: Speed,
    vehicle_length_metres: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests for `get_sectors`.
    mod test_get_sectors {
        use super::*;
        use bevy::ecs::system::SystemState;

        fn spawn_segment(
            world: &mut World,
            arm_index: usize,
            lane_index: usize,
            component: impl Component,
        ) -> Entity {
            let dummy_arm_id = Entity::PLACEHOLDER;
            let dummy_curve = StraightLinePoints([Vec3::ZERO, Vec3::ZERO]);
            let dummy_connection = Connection::Direct {
                next_segment_id: Entity::PLACEHOLDER,
            };
            let dummy_speed_limit = Speed::ZERO;

            let segment = Segment::new(
                dummy_curve,
                dummy_arm_id,
                arm_index,
                lane_index,
                dummy_connection,
                dummy_speed_limit,
            );

            world.spawn((segment, component)).id()
        }

        #[test]
        fn get_sectors_filters_matching_arms_and_lanes() {
            let mut world = World::new();

            // Entry arm index 1, number of arms 4 => prev_arm_index = (1 + 4 - 1) % 4 = 0.
            let entry_arm = 1;
            let number_of_arms = 4;

            // Target IntraArmSector entities (arm_index = 1).
            let intra_target_lane0 = spawn_segment(&mut world, 1, 0, segment_type::IntraArmSector);
            let intra_target_lane1 = spawn_segment(&mut world, 1, 1, segment_type::IntraArmSector);
            let intra_ignored_arm = spawn_segment(&mut world, 2, 0, segment_type::IntraArmSector);

            // Target InterArmSector entities (arm_index = 0).
            let inter_target_lane0 = spawn_segment(&mut world, 0, 0, segment_type::InterArmSector);
            let inter_target_lane1 = spawn_segment(&mut world, 0, 1, segment_type::InterArmSector);
            let inter_ignored_arm = spawn_segment(&mut world, 1, 0, segment_type::InterArmSector);

            let mut system_state = SystemState::<(
                Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
                Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
            )>::new(&mut world);

            let (intra_query, inter_query) = system_state.get(&world).unwrap();

            let (intra_map, inter_map) =
                get_sectors(entry_arm, number_of_arms, intra_query, inter_query);

            assert_eq!(intra_map.len(), 2);
            assert_eq!(intra_map.get_by_left(&intra_target_lane0), Some(&0));
            assert_eq!(intra_map.get_by_left(&intra_target_lane1), Some(&1));
            assert!(!intra_map.contains_left(&intra_ignored_arm));

            assert_eq!(inter_map.len(), 2);
            assert_eq!(inter_map.get_by_left(&inter_target_lane0), Some(&0));
            assert_eq!(inter_map.get_by_left(&inter_target_lane1), Some(&1));
            assert!(!inter_map.contains_left(&inter_ignored_arm));
        }

        #[test]
        fn get_sectors_handles_arm_zero_wraparound() {
            let mut world = World::new();

            // Entry arm index 0, number of arms 4 => prev_arm_index = (0 + 4 - 1) % 4 = 3.
            let entry_arm = 0;
            let number_of_arms = 4;

            let intra_arm0_lane0 = spawn_segment(&mut world, 0, 0, segment_type::IntraArmSector);
            let inter_arm3_lane2 = spawn_segment(&mut world, 3, 2, segment_type::InterArmSector);

            let mut system_state = SystemState::<(
                Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
                Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
            )>::new(&mut world);

            let (intra_query, inter_query) = system_state.get(&world).unwrap();

            let (intra_map, inter_map) =
                get_sectors(entry_arm, number_of_arms, intra_query, inter_query);

            assert_eq!(intra_map.len(), 1);
            assert_eq!(intra_map.get_by_left(&intra_arm0_lane0), Some(&0));

            assert_eq!(inter_map.len(), 1);
            assert_eq!(inter_map.get_by_left(&inter_arm3_lane2), Some(&2));
        }

        #[test]
        fn get_sectors_returns_empty_maps_when_no_matching_sectors_exist() {
            let mut world = World::new();

            // Spawn segments for arm 2, but query for arm 0.
            spawn_segment(&mut world, 2, 0, segment_type::IntraArmSector);
            spawn_segment(&mut world, 2, 0, segment_type::InterArmSector);

            let mut system_state = SystemState::<(
                Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
                Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
            )>::new(&mut world);

            let (intra_query, inter_query) = system_state.get(&world).unwrap();

            let (intra_map, inter_map) = get_sectors(0, 4, intra_query, inter_query);

            assert!(intra_map.is_empty());
            assert!(inter_map.is_empty());
        }

        #[test]
        fn get_sectors_handles_two_arm_roundabout() {
            let mut world = World::new();

            // Entry arm index 0, number of arms 2 => prev_arm_index = (0 + 2 - 1) % 2 = 1.
            let entry_arm = 0;
            let number_of_arms = 2;

            let intra_arm0_lane0 = spawn_segment(&mut world, 0, 0, segment_type::IntraArmSector);
            let inter_arm1_lane0 = spawn_segment(&mut world, 1, 0, segment_type::InterArmSector);

            let mut system_state = SystemState::<(
                Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
                Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
            )>::new(&mut world);

            let (intra_query, inter_query) = system_state.get(&world).unwrap();

            let (intra_map, inter_map) =
                get_sectors(entry_arm, number_of_arms, intra_query, inter_query);

            assert_eq!(intra_map.len(), 1);
            assert_eq!(intra_map.get_by_left(&intra_arm0_lane0), Some(&0));

            assert_eq!(inter_map.len(), 1);
            assert_eq!(inter_map.get_by_left(&inter_arm1_lane0), Some(&0));
        }

        #[test]
        fn get_sectors_ignores_other_segment_types_on_same_arm() {
            let mut world = World::new();

            let entry_arm = 1;
            let number_of_arms = 4;

            // Target sectors
            let intra_target = spawn_segment(&mut world, 1, 0, segment_type::IntraArmSector);
            let inter_target = spawn_segment(&mut world, 0, 0, segment_type::InterArmSector);

            // Non-sector segments on the target arms.
            spawn_segment(&mut world, 1, 0, segment_type::EntryLine);
            spawn_segment(&mut world, 0, 0, segment_type::EntryDeflection);

            let mut system_state = SystemState::<(
                Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
                Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
            )>::new(&mut world);

            let (intra_query, inter_query) = system_state.get(&world).unwrap();

            let (intra_map, inter_map) =
                get_sectors(entry_arm, number_of_arms, intra_query, inter_query);

            assert_eq!(intra_map.len(), 1);
            assert!(intra_map.contains_left(&intra_target));

            assert_eq!(inter_map.len(), 1);
            assert!(inter_map.contains_left(&inter_target));
        }
    }

    /// Tests for `should_yield_at_entry`.
    mod test_should_yield_at_entry {
        use super::*;

        const DEFAULT_CRITICAL_GAP: Duration = Duration::new(3, 0);

        /// Helper to construct a `CirculatingVehicleInfo` instance.
        fn create_circulating_vehicle(
            distance_metres: f32,
            speed_mps: f32,
        ) -> CirculatingVehicleInfo {
            CirculatingVehicleInfo {
                distance_to_conflict_metres: distance_metres,
                speed: Speed::try_new_metres_per_second(speed_mps).unwrap(),
                vehicle_length_metres: 4.5,
            }
        }

        #[test]
        fn yields_when_time_difference_is_within_critical_gap() {
            // Distance 15m @ 10 m/s -> TTA = 1.5s (< 3.0s critical gap) -> Must yield.
            let circulating_vehicles = vec![create_circulating_vehicle(15.0, 10.0)];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(should_yield);
        }

        #[test]
        fn does_not_yield_when_time_difference_exceeds_critical_gap() {
            // Distance 40m @ 10 m/s -> TTA = 4.0s (>= 3.0s critical gap) -> Safe to proceed.
            let circulating_vehicles = vec![create_circulating_vehicle(40.0, 10.0)];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(!should_yield);
        }

        #[test]
        fn ignores_circulating_vehicles_that_have_cleared_the_conflict_point() {
            // Distance < 0.0 indicates the vehicle has already cleared the conflict point.
            let circulating_vehicles = vec![create_circulating_vehicle(-5.0, 10.0)];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(!should_yield);
        }

        #[test]
        fn yields_when_vehicle_is_queued_near_conflict_zone() {
            // Distance < 12.0m and speed < 0.5 m/s -> Queued vehicle near conflict point -> Must yield.
            let circulating_vehicles = vec![create_circulating_vehicle(8.0, 0.2)];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(should_yield);
        }

        #[test]
        fn returns_false_when_no_circulating_vehicles_present() {
            let circulating_vehicles = vec![];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(!should_yield);
        }
    }
}
