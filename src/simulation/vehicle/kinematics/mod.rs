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

/// Calculates vehicles' accelerations due to the IDM model, road geometry, and yield logic.
pub(in crate::simulation) fn calculate_accelerations(
    roundabout_blueprint: Res<RoundaboutBlueprint>,
    conflict_points: Res<RoundaboutConflictPoints>,
    segments: Query<&Segment>,
    entry_line_segments: Query<&Segment, With<segment_type::EntryLine>>,
    sector_segments: Query<
        &Segment,
        Or<(
            With<segment_type::IntraArmSector>,
            With<segment_type::InterArmSector>,
        )>,
    >,
    vehicles: Query<(Entity, &IdmDriver, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
    mut accelerations: Query<&mut Acceleration, With<Vehicle>>,
    lead_vehicles_query: Query<(Entity, &Navigator, &Speed), With<Vehicle>>,
) {
    for (id, idm_driver, kinematics, navigator, &speed) in vehicles {
        let lead_vehicle_info = find_lead_vehicle(&segments, &lead_vehicles_query, id).ok();

        let current_segment_id = navigator.current_segment_id();
        // If the vehicle is on the entry line, then check if it needs to yield.
        if let Ok(segment) = entry_line_segments.get(current_segment_id) {
            let distance_to_line =
                Distance::try_new_metres((1.0 - navigator.progress()) * segment.length())
                    .expect("expected distance to be positive");

            let mut circulating_vehicles: Vec<Entity> = Vec::new();
            for (sector_id, sector_idm_driver, sector_kinematics, sector_navigator, sector_speed) in
                vehicles
            {
                if sector_id == id {
                    continue;
                } else if let Ok(sector_segment) =
                    sector_segments.get(sector_navigator.current_segment_id())
                {
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

        let new_acceleration = Acceleration::new(raw_acceleration.clamp(
            *kinematics.max_deceleration(),
            *kinematics.max_acceleration(),
        ));

        if let Ok(mut acceleration) = accelerations.get_mut(id) {
            *acceleration = new_acceleration;
        }
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
        let delta_progress = (*speed * delta_seconds) / current_segment.length();
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

/// ### Returns
/// `Ok(Vec<(usize, Distance, Speed, Acceleration)>)`.
/// * The `usize` is the lane index of the vehicle.
/// * The `Distance` is the vehicle's distance along `entry_arm_index - 1`'s inter arm sector
/// (+ the distance along `entry_arm_index`'s intra arm sector if the vehicle is on that sector).
fn gather_circulating_vehicles(
    this_vehicle_id: Entity,
    entry_arm_index: usize,
    number_of_arms: usize,
    intra_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
    vehicles: &Query<(Entity, &Navigator, &Speed, &Acceleration), With<Vehicle>>,
) -> Result<Vec<(usize, Distance, Speed, Acceleration)>, String> {
    let (intra_arm_sectors, inter_arm_sectors) = get_sectors(
        entry_arm_index,
        number_of_arms,
        intra_arm_sectors_query,
        inter_arm_sectors_query,
    );

    let mut circulating_vehicles: Vec<(usize, Distance, Speed, Acceleration)> = Vec::new();

    for (id, navigator, &speed, &acceleration) in vehicles {
        if id == this_vehicle_id {
            continue;
        }

        let current_segment_id = navigator.current_segment_id();

        if let Some(&lane_index) = intra_arm_sectors.get_by_left(&current_segment_id) {
            let &inter_arm_id = inter_arm_sectors.get_by_right(&lane_index).ok_or_else(|| {
                format!("expected to find inter arm sector for lane {lane_index}")
            })?;

            let (_, inter_arm_segment) = inter_arm_sectors_query
                .get(inter_arm_id)
                .map_err(|_| format!("expected segment for inter arm entity {inter_arm_id:?}"))?;

            let (_, intra_arm_segment) =
                intra_arm_sectors_query
                    .get(current_segment_id)
                    .map_err(|_| {
                        format!("expected segment for intra arm entity {current_segment_id:?}")
                    })?;

            let distance_metres =
                inter_arm_segment.length() + intra_arm_segment.length() * navigator.progress();
            let distance = Distance::try_new_metres(distance_metres)
                .map_err(|error| format!("invalid distance calculated: {error:?}"))?;

            circulating_vehicles.push((lane_index, distance, speed, acceleration));
        } else if let Some(&lane_index) = inter_arm_sectors.get_by_left(&current_segment_id) {
            let (_, inter_arm_segment) =
                inter_arm_sectors_query
                    .get(current_segment_id)
                    .map_err(|_| {
                        format!("expected segment for inter arm entity {current_segment_id:?}")
                    })?;

            let distance_metres = inter_arm_segment.length() * navigator.progress();
            let distance = Distance::try_new_metres(distance_metres)
                .map_err(|error| format!("invalid distance calculated: {error:?}"))?;

            circulating_vehicles.push((lane_index, distance, speed, acceleration));
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
    vehicles: &Query<(Entity, &Navigator, &Speed), With<Vehicle>>,
    this_vehicle_id: Entity,
) -> Result<LeadVehicleInfo, String> {
    let (_, this_navigator, _) = vehicles
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
    for (vehicle_id, navigator, _) in vehicles {
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

    let (_, _, lead_speed) = vehicles
        .get(lead_vehicle_id)
        .expect("expected to find vehicle components");

    Ok(LeadVehicleInfo {
        distance: total_distance,
        speed: *lead_speed,
    })
}

/// Compares the time-to-arrival (TTA) of the entering and the circulating vehicles.
///
/// Decides whether the vehicle entering is required to yield at the line.
///
/// ### Arguments
/// * `circulating_vehicles` - The `Distance` is the distance along Arm N-1's inter arm sector.
/// * `arm_index` - The arm index that the entry vehicle is on and that the circulating vehicles can approach.
/// * `entry_vehicle_distance_to_line` - The distance that the entry vehicle is to the start of the deflection curve / yield line.
/// * `critical_gap` - The minimum amount of time the entry vehicle requires to enter between circulating traffic.
fn should_yield_at_entry(
    conflict_points: &RoundaboutConflictPoints,
    number_of_lanes: usize,
    circulating_vehicles: &[(Distance, Speed, Acceleration)],
    arm_index: usize,
    entry_lane_index: usize,
    entry_vehicle_speed: Speed,
    entry_vehicle_acceleration: Acceleration,
    entry_vehicle_distance_to_line: Distance,
    critical_gap: Duration,
) -> bool {
    // Check each circulating lane that overlaps the entry vehicle's path.
    for circulating_lane_index in 0..number_of_lanes {
        let Some((conflict_point_index, _)) =
            ConflictPointIndex::try_new(arm_index, entry_lane_index, circulating_lane_index)
        else {
            return false;
        };

        // If None, then these lanes do not overlap.
        if let Some(conflict_point) = conflict_points.get(conflict_point_index) {
            // Entry vehicle's distance to conflict point.
            let total_entry_distance = Distance::try_new_metres(
                *entry_vehicle_distance_to_line + *conflict_point.entry_distance_to_point,
            )
            .expect("expected to have a positive distance");

            let entry_tta = Kinematics::calculate_time_to_arrival(
                entry_vehicle_speed,
                entry_vehicle_acceleration,
                total_entry_distance,
            );

            for &(circulating_distance, circulating_speed, circulating_acceleration) in
                circulating_vehicles
            {
                // Circulating vehicle's distance to conflict point.
                let Ok(total_circulating_distance) = Distance::try_new_metres(
                    *conflict_point.sector_distance_to_point - *circulating_distance,
                ) else {
                    continue;
                };

                let circulating_tta = Kinematics::calculate_time_to_arrival(
                    circulating_speed,
                    circulating_acceleration,
                    total_circulating_distance,
                );

                if (entry_tta - circulating_tta).as_secs_f32().abs() < critical_gap.as_secs_f32() {
                    return true;
                }
            }
        }
    }

    false
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
}
