//! Related to vehicle movement, such as `move_vehicles` system.

use crate::*;

pub(crate) mod types;

pub(crate) use types::*;

pub(super) struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TypesPlugin);
    }
}

pub(in crate::simulation) fn move_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    roundabout_blueprint: Res<RoundaboutBlueprint>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    mut vehicle_params: ParamSet<(
        Query<(Entity, &Kinematics, &Navigator)>,
        Query<(Entity, &IdmDriver, &Kinematics, &Navigator)>,
        Query<(Entity, &mut Kinematics, &mut Navigator, &mut Transform)>,
    )>,
) {
    let delta_seconds = time.delta_secs();

    // Collect driver and kinematic values to release borrow on vehicle_params (so it can be used in the loop).
    let vehicle_drivers = vehicle_params
        .p1()
        .iter()
        .map(|(id, idm_driver, kinematics, _)| (id, *idm_driver, *kinematics))
        .collect::<Vec<_>>();

    // let mut accelerations = Vec::with_capacity(vehicle_drivers.len());
    for (id, idm_driver, kinematics) in vehicle_drivers {
        let lead_vehicle_info = find_lead_vehicle(&segments, &vehicle_params.p0(), id).ok();
        let acceleration = idm_driver.calculate_acceleration(
            kinematics.speed,
            roundabout_blueprint.speed_limit(),
            lead_vehicle_info,
        );

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
    circulating_vehicles: &[(&Kinematics, Distance)],
    arm_index: usize,
    entry_lane_index: usize,
    entry_vehicle_kinematics: &Kinematics,
    entry_vehicle_distance_to_line: Distance,
    critical_gap: Duration,
) -> bool {
    // Check each circulating lane that overlaps the entry vehicle's path.
    for circulating_lane_index in 0..number_of_lanes {
        // If None, then these lanes do not overlap.
        if let Some(conflict_point) = conflict_points.get(ConflictPointIndex {
            arm_index,
            entry_lane_index,
            circulating_lane_index,
        }) {
            // Entry vehicle's distance to conflict point.
            let total_entry_distance = Distance::try_new(
                *entry_vehicle_distance_to_line + *conflict_point.entry_distance_to_point,
            )
            .expect("expected to have a positive distance");

            let entry_tta =
                entry_vehicle_kinematics.calculate_time_to_arrival(total_entry_distance);

            for &(circulating_kinematics, circulating_distance) in circulating_vehicles {
                // Circulating vehicle's distance to conflict point.
                let Ok(total_circulating_distance) = Distance::try_new(
                    *conflict_point.sector_distance_to_point - *circulating_distance,
                ) else {
                    continue;
                };

                let circulating_tta =
                    circulating_kinematics.calculate_time_to_arrival(total_circulating_distance);

                if (entry_tta - circulating_tta).as_secs_f32().abs() < critical_gap.as_secs_f32() {
                    return true;
                }
            }
        }
    }

    false
}
