//! Contains the instructions from assembling a roundabout layout from blueprints.

use crate::*;

pub(super) struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Assembles the roundabout using the blueprint resources.
/// Removes the existing layout and vehicles before spawning the new layout.
// pub(crate) due to use in test code.
pub(crate) fn assemble_roundabout(
    mut commands: Commands,
    roundabout_blueprint: Res<RoundaboutBlueprint>,
) {
    info!("Assembling roundabout from blueprints.");

    commands.run_system_cached(clear_existing_layout);

    let arm_blueprints = roundabout_blueprint.arm_blueprints();
    let number_of_arms = arm_blueprints.len();

    let circle_blueprint = roundabout_blueprint.circle_blueprint();
    let inner_radius = circle_blueprint.radius();
    let deflection_radius = circle_blueprint.deflection_radius();

    let number_of_lanes = roundabout_blueprint.number_of_lanes();

    let roundabout_topology =
        RoundaboutTopology::new(&mut commands, number_of_lanes, number_of_arms);
    for (arm_index, arm_blueprint) in arm_blueprints.iter().enumerate() {
        let next_arm_index = (arm_index + 1) % number_of_arms;
        let next_arm_angle = arm_blueprints[next_arm_index].angle();

        let arm = Arm::new(arm_index, arm_blueprint.angle());
        let arm_id = roundabout_topology.get_arm_id_at(arm_index);
        // Add the ArmBundle.
        commands.entity(arm_id).insert(ArmBundle::new(arm));

        // The key is the lane index.
        let mut arm_flow_rates = HashMap::<usize, FlowRates>::new();
        for other_arm_index in 0..number_of_arms {
            let other_arm = Arm::new(other_arm_index, arm_blueprints[other_arm_index].angle());
            let other_arm_id = roundabout_topology.get_arm_id_at(other_arm_index);
            let lane_index = select_lane_index(&arm, &other_arm, number_of_arms, number_of_lanes);
            arm_flow_rates
                .entry(lane_index)
                .or_insert_with(FlowRates::new)
                .insert(other_arm_id, Frequency::new::<per_hour>(800.0));
        }

        let speed_limit_override = arm_blueprint.speed_limit_override();

        for lane_index in 0..number_of_lanes {
            // A unique identifier for naming purposes.
            // Serves no functional purpose for simulation, other than
            // being able to identify related Entities in the inspector.
            let unique_identifier = format!("[{arm_index}, {lane_index}]");
            let ids = roundabout_topology.get_ids_for(arm_index, lane_index, next_arm_index);

            let entry_geometry = LaneGeometry::generate_entry(
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );
            let (entry_line_points, entry_deflection_points) = entry_geometry.into_curves();

            commands.entity(ids.entry_deflection).insert((
                Name::new(format!("EntryDeflection {unique_identifier}")),
                segment_type::EntryDeflection,
                Segment::new(
                    entry_deflection_points,
                    arm_id,
                    arm_index,
                    lane_index,
                    Connection::Merge {
                        next_segment_id: ids.inter_arm_sector,
                    },
                    None,
                ),
            ));

            commands.entity(ids.entry_line).insert((
                Name::new(format!("EntryLine {unique_identifier}")),
                segment_type::EntryLine::new(arm_flow_rates.remove(&lane_index).unwrap()),
                Segment::new(
                    entry_line_points,
                    arm_id,
                    arm_index,
                    lane_index,
                    Connection::Direct {
                        next_segment_id: ids.entry_deflection,
                    },
                    speed_limit_override,
                ),
            ));

            commands.spawn((
                Name::new(format!("SpawnPoint {unique_identifier}")),
                SpawnPoint::new(arm_id, lane_index, ids.entry_line),
            ));

            let exit_geometry = LaneGeometry::generate_exit(
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );
            let (exit_line_points, exit_deflection_points) = exit_geometry.into_curves();

            let end_point_id = commands
                .spawn((
                    Name::new(format!("EndPoint {unique_identifier}")),
                    EndPoint::new(arm_id, lane_index),
                ))
                .id();

            commands.entity(ids.exit_line).insert((
                Name::new(format!("ExitLine {unique_identifier}")),
                segment_type::ExitLine,
                Segment::new(
                    exit_line_points,
                    arm_id,
                    arm_index,
                    lane_index,
                    Connection::EndPoint { end_point_id },
                    speed_limit_override,
                ),
            ));

            commands.entity(ids.exit_deflection).insert((
                Name::new(format!("ExitDeflection {unique_identifier}")),
                segment_type::ExitDeflection,
                Segment::new(
                    exit_deflection_points,
                    arm_id,
                    arm_index,
                    lane_index,
                    Connection::Direct {
                        next_segment_id: ids.exit_line,
                    },
                    None,
                ),
            ));

            let intra_arm_sector_geometry = SectorGeometry::generate_intra_arm(
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(ids.intra_arm_sector).insert((
                Name::new(format!("IntraArmSector {unique_identifier}")),
                segment_type::IntraArmSector,
                Segment::new(
                    intra_arm_sector_geometry,
                    arm_id,
                    arm_index,
                    lane_index,
                    Connection::Direct {
                        next_segment_id: ids.inter_arm_sector,
                    },
                    None,
                ),
            ));

            let inter_arm_sector_geometry = SectorGeometry::generate_inter_arm(
                arm_blueprint.angle(),
                next_arm_angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(ids.inter_arm_sector).insert((
                Name::new(format!("InterArmSector {unique_identifier}")),
                segment_type::InterArmSector,
                Segment::new(
                    inter_arm_sector_geometry,
                    arm_id,
                    arm_index,
                    lane_index,
                    Connection::Diverge {
                        exit_arm_index: next_arm_index,
                        exit_segment_id: ids.next_exit_deflection,
                        circulating_segment_id: ids.next_intra_arm_sector,
                    },
                    None,
                ),
            ));
        }
    }
}

/// Issues eviction notices to all entities part of the previous blueprint designs.
fn clear_existing_layout(
    mut commands: Commands,
    existing_vehicles: Query<Entity, With<Vehicle>>,
    existing_arms: Query<Entity, With<Arm>>,
    existing_segments: Query<Entity, With<Segment>>,
    existing_spawns: Query<Entity, With<SpawnPoint>>,
    existing_ends: Query<Entity, With<EndPoint>>,
) {
    // Despawn old features before assembling new layout.
    info!("Despawning old layout.");
    for entity in existing_vehicles
        .iter()
        .chain(existing_arms.iter())
        .chain(existing_segments.iter())
        .chain(existing_spawns.iter())
        .chain(existing_ends.iter())
    {
        commands.entity(entity).despawn();
    }
}

/// Returns the valid lane index to use to get from `entry_arm` to `exit_arm`.
pub(crate) fn select_lane_index(
    entry_arm: &Arm,
    exit_arm: &Arm,
    number_of_arms: usize,
    number_of_lanes: usize,
) -> usize {
    // Single-lane roundabouts always use lane 0.
    if number_of_lanes <= 1 {
        return 0;
    }

    let exit_rank = get_exit_rank(entry_arm, exit_arm, number_of_arms);
    let max_rank = number_of_arms - 1;

    // Clamp exit_rank so U-turns share highest rank with final exit.
    let rank = if exit_rank == 0 || exit_rank > max_rank {
        max_rank
    } else {
        exit_rank
    };

    let raw_progress = (rank - 1) as f32 / (max_rank - 1) as f32;
    // Adds quadratic bias (which delays using more inner lanes until later ranks).
    let biased_progress = raw_progress.powf(2.0);

    let inner_offset = (biased_progress * (number_of_lanes - 1) as f32).round() as usize;

    (number_of_lanes - 1) - inner_offset
}

/// Returns a 1-based exit rank for a vehicle travelling from `entry_arm` to `exit_arm`.
const fn get_exit_rank(entry_arm: &Arm, exit_arm: &Arm, number_of_arms: usize) -> usize {
    (exit_arm.index() + number_of_arms - entry_arm.index()) % number_of_arms
}

/// Points to all of the entities forming the roundabout.
struct RoundaboutTopology {
    arm_topologies: Vec<ArmTopology>,
}

impl RoundaboutTopology {
    fn new(commands: &mut Commands, number_of_lanes: usize, number_of_arms: usize) -> Self {
        let arm_topologies = (0..number_of_arms)
            .map(|_| ArmTopology {
                id: commands.spawn_empty().id(),
                arm_lane_topologies: (0..number_of_lanes)
                    .map(|_| ArmLaneTopology {
                        entry_line_id: commands.spawn_empty().id(),
                        entry_deflection_id: commands.spawn_empty().id(),
                        exit_line_id: commands.spawn_empty().id(),
                        exit_deflection_id: commands.spawn_empty().id(),
                        circulating_sector: CirculatingSector {
                            intra_id: commands.spawn_empty().id(),
                            inter_id: commands.spawn_empty().id(),
                        },
                    })
                    .collect(),
            })
            .collect();

        RoundaboutTopology { arm_topologies }
    }

    fn get_arm_id_at(&self, arm_index: usize) -> Entity {
        self.arm_topologies[arm_index].id
    }

    fn get_ids_for(
        &self,
        arm_index: usize,
        lane_index: usize,
        next_arm_index: usize,
    ) -> CurrentIterationIds {
        let arm_lane_topology = &self.arm_topologies[arm_index].arm_lane_topologies[lane_index];
        let next_arm_lane_topology =
            &self.arm_topologies[next_arm_index].arm_lane_topologies[lane_index];
        CurrentIterationIds {
            entry_line: arm_lane_topology.entry_line_id,
            entry_deflection: arm_lane_topology.entry_deflection_id,
            exit_line: arm_lane_topology.exit_line_id,
            exit_deflection: arm_lane_topology.exit_deflection_id,
            intra_arm_sector: arm_lane_topology.circulating_sector.intra_id,
            inter_arm_sector: arm_lane_topology.circulating_sector.inter_id,
            next_exit_deflection: next_arm_lane_topology.exit_deflection_id,
            next_intra_arm_sector: next_arm_lane_topology.circulating_sector.intra_id,
        }
    }
}

/// Points to the entities of segments associated with an arm.
///
/// Includes the intra and inter sectors on the roundabout surrounding the entry lanes.
struct ArmTopology {
    /// The arm entity itself holding an `Arm` component.
    id: Entity,
    /// The index is the lane_index.
    arm_lane_topologies: Vec<ArmLaneTopology>,
}

/// Points to the entities of segments associated with a single lane of an arm.
struct ArmLaneTopology {
    entry_line_id: Entity,
    entry_deflection_id: Entity,
    exit_line_id: Entity,
    exit_deflection_id: Entity,
    circulating_sector: CirculatingSector,
}

/// Points to the entities of segments associated with a sector of the circle.
struct CirculatingSector {
    /// Between Arm N's exit and Arm N's entry.
    intra_id: Entity,
    /// Between Arm N's entry and Arm N + 1's exit.
    inter_id: Entity,
}

struct CurrentIterationIds {
    entry_line: Entity,
    entry_deflection: Entity,
    exit_line: Entity,
    exit_deflection: Entity,
    intra_arm_sector: Entity,
    inter_arm_sector: Entity,
    next_exit_deflection: Entity,
    next_intra_arm_sector: Entity,
}
