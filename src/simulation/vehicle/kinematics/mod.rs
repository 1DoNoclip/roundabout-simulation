//! Related to vehicle movement, such as `move_vehicles` system.

use crate::*;
use bimap::BiHashMap;
use uom::ConstZero;

pub(super) struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Calculates vehicles' accelerations due to the IDM model, road geometry, and yield logic.
#[allow(clippy::too_many_arguments)]
pub(in crate::simulation) fn calculate_accelerations(
    roundabout_blueprint: Res<RoundaboutBlueprint>,
    conflict_points: Res<RoundaboutConflictPoints>,
    yield_points: Res<RoundaboutYieldPoints>,
    segments: Query<&Segment>,
    entry_line_segments: Query<&Segment, With<segment_type::EntryLine>>,
    entry_deflection_segments: Query<(Entity, &Segment), With<segment_type::EntryDeflection>>,
    // Used to check if a segment is an entry deflection segment.
    exit_deflection_segments: Query<(), With<segment_type::ExitDeflection>>,
    intra_arm_sectors: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    inter_arm_sectors: Query<(Entity, &Segment), With<segment_type::InterArmSector>>,
    vehicles: Query<(Entity, &IdmDriver, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
    circulating_vehicles: Query<(Entity, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
    mut next_accelerations: Query<&mut NextAcceleration, With<Vehicle>>,
    lead_vehicles_query: Query<(Entity, &Kinematics, &Navigator, &Speed), With<Vehicle>>,
) {
    for (id, idm_driver, kinematics, navigator, &speed) in vehicles {
        let mut lead_vehicle_info = find_lead_vehicle(&segments, &lead_vehicles_query, id).ok();
        let current_segment_id = navigator.current_segment_id();
        let current_segment = segments.get(current_segment_id).expect("expected matching segment to segment Entity");

        let yield_context = YieldContext::get(
            &yield_points,
            entry_line_segments,
            entry_deflection_segments,
            navigator,
            current_segment_id,
        );

        if let Some(context) = yield_context
            // Yield distance must be positive (else the vehicle is past the yield line)
            // and therefore we completely disregard using a virtual lead vehicle.
            && let Ok(distance_to_yield) = Distance::try_new(context.distance_to_yield)
                && distance_to_yield.get::<meter>() > 0.0
                && let Ok(circulating_vehicles) = get_circulating_vehicles(
                    id,
                    context.entry_lane_index,
                    context.entry_arm_index,
                    roundabout_blueprint.number_of_arms(),
                    &conflict_points,
                    exit_deflection_segments,
                    intra_arm_sectors,
                    inter_arm_sectors,
                    circulating_vehicles,
                )
        && should_yield_at_entry(&circulating_vehicles, idm_driver.critical_gap())
        {
            let virtual_yield_vehicle = LeadVehicleInfo {
                vehicle_kind: VehicleKind::Virtual,
                distance: distance_to_yield,
                speed: Speed::ZERO,
            };

            lead_vehicle_info = match lead_vehicle_info {
                Some(lead_vehicle_info) if *lead_vehicle_info.distance < *distance_to_yield => {
                    Some(lead_vehicle_info)
                }
                _ => Some(virtual_yield_vehicle),
            };
        }

        let kappa = get_kappa(speed, idm_driver, navigator, &segments);

        let target_speed: Velocity = if kappa > 1e-5 {
            let lateral_acceleration = idm_driver
                .comfortable_lateral_acceleration()
                .get::<meter_per_second_squared>();

            let max_cornering_speed: Velocity =
                Velocity::new::<meter_per_second>((lateral_acceleration / kappa).sqrt());

            kinematics
                .target_speed()
                .min(current_segment.speed_limit_override())
                .min(max_cornering_speed)
        } else {
            kinematics
                .target_speed()
                .min(*roundabout_blueprint.speed_limit())
        };

        let raw_acceleration: Acceleration = idm_driver.calculate_acceleration(
            speed,
            Speed::try_new(target_speed).unwrap(),
            lead_vehicle_info,
        );
        println!("{:?}", roundabout_blueprint.speed_limit());

        // Clamp within max and min vehicle values.
        let new_acceleration: Acceleration = raw_acceleration
            .max(kinematics.max_deceleration())
            .min(kinematics.max_acceleration());

        if let Ok(mut next_acceleration) = next_accelerations.get_mut(id) {
            *next_acceleration = NextAcceleration::from(new_acceleration);
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

    let total_distance = Distance::try_new(if lead_index == 0 {
        let segment_length = segments
            .get(existing_route[0])
            .map_err(|_| "expected to get segment component")?
            .length();
        (lead_progress - this_progress) * segment_length
    } else {
        let mut total_distance = Length::ZERO;

        let first_segment_length = segments
            .get(existing_route[0])
            .map_err(|_| "expected to get segment component")?
            .length();
        total_distance += (1.0 - this_progress) * first_segment_length;

        for &id in existing_route.iter().take(lead_index).skip(1) {
            let segment_length = segments
                .get(id)
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

    let (_, kinematics, _, lead_speed) = vehicles
        .get(lead_vehicle_id)
        .expect("expected to find vehicle components");

    Ok(LeadVehicleInfo {
        vehicle_kind: VehicleKind::Real(kinematics.vehicle_length()),
        distance: total_distance,
        speed: *lead_speed,
    })
}

struct YieldContext {
    entry_arm_index: usize,
    entry_lane_index: usize,
    distance_to_yield: Length,
}

impl YieldContext {
    fn get(
        yield_points: &Res<RoundaboutYieldPoints>,
        entry_line_segments: Query<&Segment, With<segment_type::EntryLine>>,
        entry_deflection_segments: Query<(Entity, &Segment), With<segment_type::EntryDeflection>>,
        navigator: &Navigator,
        current_segment_id: Entity,
    ) -> Option<Self> {
        let yield_context = if let Ok(entry_segment) = entry_line_segments.get(current_segment_id) {
            let (deflection_id, deflection_segment) = entry_deflection_segments
                .iter()
                .find(|&(_, segment)| {
                    segment.arm_id() == entry_segment.arm_id()
                        && segment.lane_index() == entry_segment.lane_index()
                })
                .expect("entry line segment should have an associated entry deflection segment");

            let arm_index = entry_segment.arm_index();
            let lane_index = entry_segment.lane_index();

            if let Some(yield_point) = yield_points.get(YieldPointIndex::new(arm_index, lane_index))
            {
                // If the yield point is on the entry line.
                let distance_to_yield = if yield_point.segment_id() == current_segment_id {
                    (yield_point.progress() - navigator.progress()) * entry_segment.length()
                }
                // If the yield point is on the entry deflection.
                else if yield_point.segment_id() == deflection_id {
                    (1.0 - navigator.progress()) * entry_segment.length()
                        + yield_point.progress() * deflection_segment.length()
                } else {
                    Length::new::<meter>(0.0)
                };
                Some((arm_index, lane_index, distance_to_yield))
            } else {
                None
            }
        } else if let Ok((deflection_id, deflection_segment)) =
            entry_deflection_segments.get(current_segment_id)
        {
            let arm_index = deflection_segment.arm_index();
            let lane_index = deflection_segment.lane_index();

            if let Some(yield_point) = yield_points.get(YieldPointIndex::new(arm_index, lane_index))
            {
                if yield_point.segment_id() == deflection_id {
                    let distance_to_yield = Length::new::<meter>(
                        (yield_point.progress() - navigator.progress())
                            * deflection_segment.length().get::<meter>(),
                    );
                    Some((arm_index, lane_index, distance_to_yield))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        yield_context.map(
            |(entry_arm_index, entry_lane_index, distance_to_yield)| YieldContext {
                entry_arm_index,
                entry_lane_index,
                distance_to_yield,
            },
        )
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

        let vehicle_length = kinematics.vehicle_length();
        let current_segment_id = navigator.current_segment_id();
        let Some(next_segment_id) = navigator.next_segment_id() else {
            // The vehicle's next segment is not a valid one, therefore
            // the current segment must be the last segment, so is not on
            // a circulating segment.
            continue;
        };
        // If the vehicle is going to exit the circle next, then ignore it.
        // Entering vehicles do not need to yield to exiting vehicles.
        if exit_deflection_segments_query.get(next_segment_id).is_ok() {
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
        let distance_to_conflict =
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

                intra_arm_segment.length() * conflict_point.intra_arm_sector_progress
                    + inter_arm_segment.length() * (1.0 - navigator.progress())
            // If on the intra arm segment.
            } else {
                let (_, intra_arm_segment) = intra_arm_sectors_query
                    .get(current_segment_id)
                    .map_err(|_| {
                        format!("expected Segment for intra arm Entity {current_segment_id:?}")
                    })?;

                intra_arm_segment.length()
                    * (conflict_point.intra_arm_sector_progress - navigator.progress())
            };

        // Retain in vector if vehicle is approaching or still clearing the conflict zone.
        if distance_to_conflict > -vehicle_length {
            circulating_vehicles.push(CirculatingVehicleInfo {
                distance_to_conflict,
                speed,
                vehicle_length,
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

/// Gets the curvature (kappa, `κ`) at the lookahead distance position.
fn get_kappa(
    current_speed: Speed,
    idm_driver: &IdmDriver,
    navigator: &Navigator,
    segments: &Query<&Segment>,
) -> f32 {
    let lookahead_distance: Length = idm_driver.geometry_time_headway() * *current_speed;
    let current_segment = segments
        .get(navigator.current_segment_id())
        .expect("expected current segment ID to be valid");
    let current_progress = navigator.progress();
    let distance_to_end: Length = (1.0 - current_progress) * current_segment.length();
    // Get the curvature of this segment.
    let (segment, progress) = if distance_to_end > lookahead_distance {
        let progress = (lookahead_distance / current_segment.length())
            .get::<uom::si::ratio::ratio>()
            + current_progress;
        (current_segment, progress)
    }
    // We need to look at ahead segments until we get to the lookahead distance.
    else {
        let route = navigator.route();
        let mut current_segment_index = navigator.current_segment_index();
        let mut remaining_distance: Length = lookahead_distance - distance_to_end;
        loop {
            current_segment_index += 1;
            let Some(&current_segment_id) = route.get(current_segment_index) else {
                return 0.0;
            };
            let current_segment = segments
                .get(current_segment_id)
                .expect("expected current segment ID to be valid");

            let progress =
                (remaining_distance / current_segment.length()).get::<uom::si::ratio::ratio>();
            if progress <= 1.0 {
                break (current_segment, progress);
            } else {
                remaining_distance -= current_segment.length();
            }
        }
    };
    segment.curvature_at(progress).abs()
}

pub(in crate::simulation) fn update_vehicle_accelerations(
    query: Query<(&mut AccelerationComponent, &NextAcceleration)>,
) {
    for (mut acceleration, &next_acceleration) in query {
        **acceleration = *next_acceleration;
    }
}

pub(in crate::simulation) fn apply_accelerations(
    time: Res<Time>,
    query: Query<(&mut Speed, &AccelerationComponent)>,
) {
    let delta_time = UomTime::new::<second>(time.delta_secs());
    for (mut speed, &acceleration) in query {
        speed.apply_acceleration(*acceleration, delta_time);
    }
}

/// Moves vehicles along their routes.
///
/// Increments segments once a vehicle has reached the end of the current segment,
/// or despawns them if they reach the end of the route.
pub(in crate::simulation) fn move_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    segments: Query<&Segment>,
    vehicles: Query<(Entity, &mut Navigator, &mut Transform, &Speed), With<Vehicle>>,
) {
    let delta_time: UomTime = UomTime::new::<second>(time.delta_secs());
    for (id, mut navigator, mut transform, &speed) in vehicles {
        let current_segment_id = navigator.current_segment_id();
        let Ok(current_segment) = segments.get(current_segment_id) else {
            warn!("Found no segment associated with segment entity.");
            continue;
        };
        let delta_progress =
            ((*speed * delta_time) / current_segment.length()).get::<uom::si::ratio::ratio>();
        match navigator.add_progress(delta_progress) {
            Ok(_) => {
                let progress = navigator.progress();

                let position = current_segment.position_at(progress);
                transform.translation = position;

                // Note: This does come at a small unnecessary cost if rendering is disabled
                // as the vehicle rotation does not matter apart from when rendering it.
                let tangent = current_segment.tangent_at(progress);
                // For 2D (XY) plane, calculate the angle from the tangent vector.
                let angle = tangent.y.atan2(tangent.x);
                transform.rotation = Quat::from_rotation_z(angle);
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

/// A vehicle's acceleration in the next frame.
/// Used to prevent runtime panics from mutable and immutable borrows of `Acceleration`.
///
/// `Acceleration` is updated to the value of `NextAcceleration` each frame.
#[derive(Clone, Component, Copy, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct NextAcceleration(#[reflect(ignore)] Acceleration);

impl From<Acceleration> for NextAcceleration {
    fn from(value: Acceleration) -> Self {
        NextAcceleration(value)
    }
}

fn should_yield_at_entry(
    circulating_vehicles: &[CirculatingVehicleInfo],
    critical_gap: UomTime,
) -> bool {
    for circulating_vehicle in circulating_vehicles {
        // Ignore vehicles that have passed the conflict zone.
        if circulating_vehicle.distance_to_conflict < -circulating_vehicle.vehicle_length {
            continue;
        }
        // If the vehicle is queued near the conflict zone then prevent entry.
        else if circulating_vehicle.distance_to_conflict <= Length::new::<meter>(8.0)
            && *circulating_vehicle.speed < Velocity::new::<meter_per_second>(0.5)
        {
            return true;
        }

        // Avoid division by zero.
        let speed = circulating_vehicle
            .speed
            .max(Velocity::new::<meter_per_second>(0.1));
        let time_to_conflict = circulating_vehicle.distance_to_conflict / speed;

        if time_to_conflict < critical_gap {
            return true;
        }
    }

    false
}

#[derive(Clone, Copy, Debug)]
struct CirculatingVehicleInfo {
    distance_to_conflict: Length,
    speed: Speed,
    vehicle_length: Length,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests for `calculate_accelerations`.
    mod test_calculate_accelerations {
        // use super::*;
        // use crate::simulation::vehicle::VehicleBundle;

        // #[test]
        // fn acceleration_clamps_to_geometry_speed() {
        //     // Because `calculate_accelerations` requires many highly specific Res and Query parameters,
        //     // testing it requires setting up a Bevy App and running a single schedule update.
        //     let mut app = App::new();

        //     // Insert required resources.
        //     app.insert_resource(
        //         RoundaboutBlueprint::try_new(
        //             vec![
        //                 ArmBlueprint::from_degrees(0.0, None, 1.0),
        //                 ArmBlueprint::from_degrees(90.0, None, 1.0),
        //                 ArmBlueprint::from_degrees(180.0, None, 1.0),
        //                 ArmBlueprint::from_degrees(270.0, None, 1.0),
        //             ],
        //             CircleBlueprint::try_new(
        //                 Length::new::<meter>(30.0),
        //                 Length::new::<meter>(10.0),
        //             )
        //             .unwrap(),
        //             2,
        //             Speed::try_new(Velocity::new::<meter_per_second>(10.0)).unwrap(),
        //         )
        //         .unwrap(),
        //     );
        //     // Insert dummy resources for YieldPoints and ConflictPoints to satisfy the system signature.
        //     app.insert_resource(RoundaboutConflictPoints::default());
        //     app.insert_resource(RoundaboutYieldPoints::default());

        //     // Spawn a vehicle and a sharp road segment.
        //     let segment_id = app
        //         .world_mut()
        //         .spawn(Segment {
        //             length: Length::new::<meter>(100.0),
        //             curvature: 0.1, // 1 / 10m radius curve
        //         })
        //         .id();

        //     let segment_id = app
        //         .world_mut()
        //         .spawn(Segment::new(
        //             // 10m radius curve.
        //             DeflectionCurvePoints([
        //                 Vec3::new(10.0, 0.0, 0.0),
        //                 Vec3::new(10.0, 5.523, 0.0),
        //                 Vec3::new(5.523, 10.0, 0.0),
        //                 Vec3::new(0.0, 10.0, 0.0),
        //             ]),
        //             Entity::PLACEHOLDER,
        //             0,
        //             0,
        //             Connection::Direct {
        //                 next_segment_id: Entity::PLACEHOLDER,
        //             },
        //             Speed::try_new(Velocity::new::<meter_per_second>(10.0)).unwrap(),
        //         ))
        //         .id();

        //     let vehicle_id = app
        //         .world_mut()
        //         .spawn(VehicleBundle::try_new(
        //             segments,
        //             current_speed,
        //             target_speed,
        //             max_acceleration,
        //             max_deceleration,
        //             route,
        //         ).unwrap())
        //         .id();

        //     let vehicle_entity = app
        //         .world_mut()
        //         .spawn((
        //             Vehicle,
        //             Speed::new::<meter_per_second>(20.0), // Currently travelling at 20 m/s.
        //             NextAcceleration::from(Acceleration::new::<meter_per_second_squared>(0.0)),
        //             IdmDriver {
        //                 geometry_time_headway: Time::new::<second>(1.0),
        //                 comfortable_lateral_acceleration: Acceleration::new::<
        //                     meter_per_second_squared,
        //                 >(2.0),
        //                 ..default()
        //             },
        //             Kinematics {
        //                 target_speed: Velocity::new::<meter_per_second>(30.0),
        //                 max_acceleration: Acceleration::new::<meter_per_second_squared>(3.0),
        //                 max_deceleration: Acceleration::new::<meter_per_second_squared>(-5.0),
        //             },
        //             Navigator {
        //                 current_segment_id: segment_id,
        //                 progress: 0.0,
        //                 route: vec![segment_id],
        //                 current_segment_index: 0,
        //             },
        //         ))
        //         .id();

        //     // Run the system.
        //     app.add_systems(Update, calculate_accelerations);
        //     app.update();

        //     // Validate output.
        //     // V_max = sqrt(lat_accel / kappa) = sqrt(2.0 / 0.1) = sqrt(20) ≈ 4.47 m/s
        //     // because current speed (20 m/s) > target cornering speed (4.47 m/s),
        //     // the IDM model should output a strong negative acceleration (braking).
        //     let next_acceleration = app.world().get::<NextAcceleration>(vehicle_entity).unwrap();

        //     assert!(
        //         next_acceleration.get::<meter_per_second_squared>() < 0.0,
        //         "Vehicle did not brake for the upcoming curve"
        //     );
        // }
    }

    /// Tests for `get_kappa`.
    mod test_get_kappa {
        use super::*;
        use bevy::ecs::system::SystemState;

        #[test]
        fn within_current_segment() {
            let mut world = World::new();

            let arm_id = world
                .spawn(Arm::new(
                    0,
                    Rot2::degrees(0.0),
                    5.0,
                    DestinationWeights::new(),
                ))
                .id();

            let segment_2_id = world
                .spawn(Segment::new(
                    StraightLinePoints([Vec3::new(50.0, 0.0, 0.0), Vec3::new(100.0, 50.0, 0.0)]),
                    arm_id,
                    0,
                    0,
                    Connection::Direct {
                        next_segment_id: Entity::PLACEHOLDER,
                    },
                    Speed::default(),
                ))
                .id();

            let segment_1_id = world
                .spawn(Segment::new(
                    StraightLinePoints([Vec3::ZERO, Vec3::new(50.0, 0.0, 0.0)]),
                    arm_id,
                    0,
                    0,
                    Connection::Direct {
                        next_segment_id: segment_2_id,
                    },
                    Speed::default(),
                ))
                .id();

            // Setup IdmDriver & Navigator.
            let idm_driver = IdmDriver::new(
                0.95,
                Acceleration::new::<meter_per_second_squared>(2.0),
                Acceleration::new::<meter_per_second_squared>(3.0),
                Acceleration::new::<meter_per_second_squared>(-2.5),
                Distance::try_new(Length::new::<meter>(5.0)).unwrap(),
                UomTime::new::<second>(1.5),
                UomTime::new::<second>(2.0),
                UomTime::new::<second>(4.0),
                4.0,
            );
            let speed = Speed::try_new(Velocity::new::<meter_per_second>(10.0)).unwrap();
            let navigator = Navigator::try_new(vec![segment_1_id, segment_2_id]).unwrap();

            // Extract the Query using SystemState.
            let mut system_state: SystemState<Query<&Segment>> = SystemState::new(&mut world);
            let segments_query = system_state.get(&world).unwrap();

            // Lookahead = 10m/s * 2s = 20m.
            // Current distance to end = (1.0 - 0.0) * 50 = 45m.
            // Lookahead is within the current segment.
            let kappa = get_kappa(speed, &idm_driver, &navigator, &segments_query);

            assert_eq!(kappa, 0.0);
        }

        // #[test]
        // fn looks_ahead_to_next_segment() {
        //     let mut world = World::new();

        //     let segment_1 = world
        //         .spawn(Segment {
        //             length: Length::new::<meter>(20.0),
        //             curvature: 0.0,
        //         })
        //         .id();

        //     let segment_2 = world
        //         .spawn(Segment {
        //             length: Length::new::<meter>(30.0),
        //             curvature: 0.5, // High curvature
        //         })
        //         .id();

        //     let speed = Speed::new::<meter_per_second>(15.0);
        //     let idm_driver = IdmDriver {
        //         geometry_time_headway: Time::new::<second>(2.0),
        //         ..default()
        //     };

        //     let navigator = Navigator {
        //         current_segment_id: segment_1,
        //         progress: 0.5, // 10m into the 20m segment. 10m remaining.
        //         route: vec![segment_1, segment_2],
        //         current_segment_index: 0,
        //     };

        //     let mut system_state: SystemState<Query<&Segment>> = SystemState::new(&mut world);
        //     let segments_query = system_state.get(&world);

        //     // Lookahead = 15m/s * 2s = 30m.
        //     // Distance remaining on segment 1 = 10m.
        //     // Remaining lookahead distance to traverse on segment 2 = 20m.
        //     // Progress on segment 2 = 20m / 30m = 0.666...
        //     let kappa = get_kappa(speed, &idm_driver, &navigator, &segments_query);

        //     assert_eq!(kappa, 0.5);
        // }

        // #[test]
        // fn route_end_fallback() {
        //     let mut world = World::new();

        //     let segment_1 = world
        //         .spawn(Segment {
        //             length: Length::new::<meter>(10.0),
        //             curvature: 0.2,
        //         })
        //         .id();

        //     let speed = Speed::new::<meter_per_second>(20.0);
        //     let idm_driver = IdmDriver {
        //         geometry_time_headway: Time::new::<second>(2.0),
        //         ..default()
        //     };

        //     let navigator = Navigator {
        //         current_segment_id: segment_1,
        //         progress: 0.5,
        //         route: vec![segment_1], // No next segment
        //         current_segment_index: 0,
        //     };

        //     let mut system_state: SystemState<Query<&Segment>> = SystemState::new(&mut world);
        //     let segments_query = system_state.get(&world);

        //     // Lookahead = 40m. Remaining segment 1 = 5m.
        //     // Overshoots route bounds. Should safely return 0.0.
        //     let kappa = get_kappa(speed, &idm_driver, &navigator, &segments_query);

        //     assert_eq!(kappa, 0.0);
        // }
    }

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
        fn filters_matching_arms_and_lanes() {
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
        fn handles_arm_zero_wraparound() {
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
        fn returns_empty_maps_when_no_matching_sectors_exist() {
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
        fn handles_two_arm_roundabout() {
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
        fn ignores_other_segment_types_on_same_arm() {
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
        use std::marker::PhantomData;

        const DEFAULT_CRITICAL_GAP: UomTime = UomTime {
            dimension: PhantomData,
            units: PhantomData,
            value: 3.0,
        };

        /// Helper to construct a `CirculatingVehicleInfo` instance.
        fn create_circulating_vehicle(
            distance: Length,
            velocity: Velocity,
        ) -> CirculatingVehicleInfo {
            CirculatingVehicleInfo {
                distance_to_conflict: distance,
                speed: Speed::try_new(velocity).expect("expected velocity to be positive or zero"),
                vehicle_length: Length::new::<meter>(4.5),
            }
        }

        #[test]
        fn yields_when_time_difference_is_within_critical_gap() {
            // Distance 15m @ 10 m/s -> TTA = 1.5s (< 3.0s critical gap) -> Must yield.
            let circulating_vehicles = vec![create_circulating_vehicle(
                Length::new::<meter>(15.0),
                Velocity::new::<meter_per_second>(10.0),
            )];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(should_yield);
        }

        #[test]
        fn does_not_yield_when_time_difference_exceeds_critical_gap() {
            // Distance 40m @ 10 m/s -> TTA = 4.0s (>= 3.0s critical gap) -> Safe to proceed.
            let circulating_vehicles = vec![create_circulating_vehicle(
                Length::new::<meter>(40.0),
                Velocity::new::<meter_per_second>(10.0),
            )];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(!should_yield);
        }

        #[test]
        fn ignores_circulating_vehicles_that_have_cleared_the_conflict_point() {
            // Distance < 0.0 indicates the vehicle has already cleared the conflict point.
            let circulating_vehicles = vec![create_circulating_vehicle(
                Length::new::<meter>(-5.0),
                Velocity::new::<meter_per_second>(10.0),
            )];

            let should_yield = should_yield_at_entry(&circulating_vehicles, DEFAULT_CRITICAL_GAP);

            assert!(!should_yield);
        }

        #[test]
        fn yields_when_vehicle_is_queued_near_conflict_zone() {
            // Distance < 12.0m and speed < 0.5 m/s -> Queued vehicle near conflict point -> Must yield.
            let circulating_vehicles = vec![create_circulating_vehicle(
                Length::new::<meter>(8.0),
                Velocity::new::<meter_per_second>(0.2),
            )];

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
