//! Related to vehicle movement, such as `move_vehicles` system.

use crate::*;
use bevy::platform::collections::HashMap;

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
                Distance::try_new((1.0 - navigator.progress()) * segment.length())
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
/// * `Ok(Vec<(Distance, Speed, Acceleration)>)` -
/// The distance is the vehicle's distance along `entry_arm_index - 1`'s inter arm sector
/// (+ the distance along `entry_arm_index`'s intra arm sector if the vehicle is on that sector).
fn gather_circulating_vehicles(
    this_vehicle_id: Entity,
    entry_arm_index: usize,
    number_of_arms: usize,
    intra_arm_sectors: &Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors: &Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
    vehicles: &Query<(Entity, &Navigator, &Speed, &Acceleration), With<Vehicle>>,
) -> Result<Vec<(Distance, Speed, Acceleration)>, String> {
    let intra_segments = intra_arm_sectors
        .iter()
        .filter(|&(_, segment)| segment.arm_index() == entry_arm_index)
    else {
        return Err("failed to get the correct intra sector segment".to_owned());
    };
    let previous_arm_index = (entry_arm_index + number_of_arms - 1) % number_of_arms;
    let inter_segments = inter_arm_sectors
        .iter()
        .filter(|&(_, segment)| segment.arm_index() == previous_arm_index)
    else {
        return Err("failed to get the correct inter sector segment".to_owned());
    };

    let mut circulating_vehicles: Vec<(Distance, Speed, Acceleration)> = Vec::new();
    for (id, navigator, &speed, &acceleration) in vehicles {
        if id == this_vehicle_id {
            continue;
        }
        let current_segment_id = navigator.current_segment_id();
        let distance = if current_segment_id == intra_segment_id {
            Distance::try_new(
                navigator.progress() * intra_segment.length() + inter_segment.length(),
            )
        } else if current_segment_id == inter_segment_id {
            Distance::try_new(navigator.progress() * inter_segment.length())
        } else {
            // This vehicle is not one we're interested in.
            continue;
        }?;
        circulating_vehicles.push((distance, speed, acceleration));
    }
    Ok(circulating_vehicles)
}

/// Gets the relevant sectors for vehicles at `arm_index` to yield to.
/// ### Returns
/// `(Vec<Entity>, Vec<Entity)` -
/// `.0` is inter arm sectors,
/// `.1` is intra arm sectors.
fn get_sectors(
    arm_index: usize,
    number_of_arms: usize,
    intra_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors_query: Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
) -> (HashMap<usize, Entity>, HashMap<usize, Entity>) {
    let intra_arm_sectors =
        intra_arm_sectors_query
            .iter()
            .fold(HashMap::new(), |mut map, (id, segment)| {
                if segment.arm_index() == arm_index {
                    map.insert(segment.lane_index(), id);
                }
                map
            });

    let prev_arm_index = (arm_index + number_of_arms - 1) % number_of_arms;
    let inter_arm_sectors =
        inter_arm_sectors_query
            .iter()
            .fold(HashMap::new(), |mut map, (id, segment)| {
                if segment.arm_index() == prev_arm_index {
                    map.insert(segment.lane_index(), id);
                }
                map
            });

    (inter_arm_sectors, intra_arm_sectors)
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
            let total_entry_distance = Distance::try_new(
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
                let Ok(total_circulating_distance) = Distance::try_new(
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
