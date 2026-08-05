use crate::*;
use std::f32::consts::PI;

pub struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Assembles the roundabout using the blueprint resources.
/// Removes the existing layout and vehicles before spawning the new layout.
pub(crate) fn assemble_roundabout(
    mut commands: Commands,
    existing_vehicles: Query<Entity, (With<Kinematics>, With<Navigator>)>,
    existing_segments: Query<Entity, With<Segment>>,
    existing_spawns: Query<Entity, With<SpawnPoint>>,
    existing_ends: Query<Entity, With<EndPoint>>,
    intersection_blueprint: Res<IntersectionBlueprint>,
    roundabout_circle_blueprint: Res<RoundaboutCircleBlueprint>,
) {
    info!("Assembling roundabout from blueprints.");

    clear_existing_layout(
        &mut commands,
        existing_vehicles,
        existing_segments,
        existing_spawns,
        existing_ends,
    );

    let number_of_lanes = intersection_blueprint.number_of_lanes();
    let inner_radius = roundabout_circle_blueprint.radius();
    let deflection_radius = intersection_blueprint.deflection_radius();
    let speed_limit = intersection_blueprint.speed_limit();
    let arm_blueprints = &intersection_blueprint.arm_blueprints();
    let number_of_arms = arm_blueprints.len();

    let roundabout_topology =
        RoundaboutTopology::new(&mut commands, number_of_lanes, number_of_arms);
    for (arm_index, arm_blueprint) in arm_blueprints.iter().enumerate() {
        let next_arm_index = (arm_index + 1) % number_of_arms;
        let next_arm_angle = arm_blueprints[next_arm_index].angle();

        let arm_id = roundabout_topology.get_arm_id_at(arm_index);
        // Add the Arm component.
        commands.entity(arm_id).insert((
            Name::new(format!("Arm [{arm_index}]")),
            Arm::new(
                arm_index,
                arm_blueprint.angle(),
                arm_blueprint.max_vehicles_per_second(),
                calculate_destination_weights(arm_blueprints, arm_index, &roundabout_topology),
            ),
        ));

        // If the arm has a speed limit override, use that instead of the intersection default speed limit.
        let speed_limit = arm_blueprint.speed_limit_override().unwrap_or(speed_limit);

        for lane_index in 0..number_of_lanes {
            // A unique identifier for naming purposes.
            // Serves no functional purpose for simulation, other than
            // being able to identify related Entities in the inspector.
            let unique_identifier = format!("[{arm_index}, {lane_index}]");
            let ids = roundabout_topology.get_ids_for(arm_index, lane_index, next_arm_index);

            let entry_geometry = LaneGeometry::generate(
                LaneType::Entry,
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(ids.entry_deflection).insert((
                Name::new(format!("EntryDeflection {unique_identifier}")),
                Segment::new(
                    CubicBezier::new([entry_geometry.deflection_curve]),
                    Connection::Merge {
                        next_segment_id: ids.inter_arm_sector,
                    },
                    speed_limit,
                ),
            ));

            commands.entity(ids.entry_line).insert((
                Name::new(format!("EntryLine {unique_identifier}")),
                Segment::new(
                    LinearSpline::new(entry_geometry.straight_line),
                    Connection::Direct {
                        next_segment_id: ids.entry_deflection,
                    },
                    speed_limit,
                ),
            ));

            commands.spawn((
                Name::new(format!("SpawnPoint {unique_identifier}")),
                SpawnPoint::new(arm_id, lane_index, ids.entry_line),
            ));

            let exit_geometry = LaneGeometry::generate(
                LaneType::Exit,
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );

            let end_point_id = commands
                .spawn((
                    Name::new(format!("EndPoint {unique_identifier}")),
                    EndPoint::new(arm_id, lane_index),
                ))
                .id();

            commands.entity(ids.exit_line).insert((
                Name::new(format!("ExitLine {unique_identifier}")),
                Segment::new(
                    LinearSpline::new(exit_geometry.straight_line),
                    Connection::EndPoint { end_point_id },
                    speed_limit,
                ),
            ));

            commands.entity(ids.exit_deflection).insert((
                Name::new(format!("ExitDeflection {unique_identifier}")),
                Segment::new(
                    CubicBezier::new([exit_geometry.deflection_curve]),
                    Connection::Direct {
                        next_segment_id: ids.exit_line,
                    },
                    speed_limit,
                ),
            ));

            let intra_arm_sector_geometry = SectorGeometry::generate(
                SectorType::IntraArm,
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(ids.intra_arm_sector).insert((
                Name::new(format!("IntraArmSector {unique_identifier}")),
                Segment::new(
                    intra_arm_sector_geometry,
                    Connection::Direct {
                        next_segment_id: ids.inter_arm_sector,
                    },
                    speed_limit,
                ),
            ));

            let inter_arm_sector_geometry = SectorGeometry::generate(
                SectorType::InterArm { next_arm_angle },
                arm_blueprint.angle(),
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(ids.inter_arm_sector).insert((
                Name::new(format!("InterArmSector {unique_identifier}")),
                Segment::new(
                    inter_arm_sector_geometry,
                    Connection::Diverge {
                        exit_arm_index: next_arm_index,
                        exit_segment_id: ids.next_exit_deflection,
                        circulating_segment_id: ids.next_intra_arm_sector,
                    },
                    speed_limit,
                ),
            ));
        }
    }
}

/// Issues eviction notices to all entities part of the previous blueprint designs.
fn clear_existing_layout(
    commands: &mut Commands,
    existing_vehicles: Query<Entity, (With<Kinematics>, With<Navigator>)>,
    existing_segments: Query<Entity, With<Segment>>,
    existing_spawns: Query<Entity, With<SpawnPoint>>,
    existing_ends: Query<Entity, With<EndPoint>>,
) {
    info!("Despawning all vehicles");
    for vehicle in existing_vehicles {
        commands.entity(vehicle).despawn();
    }

    // Despawn old segments before assembling new layout.
    info!("Despawning all segments, spawn points and end points");
    for entity in existing_segments
        .iter()
        .chain(existing_spawns.iter())
        .chain(existing_ends.iter())
    {
        commands.entity(entity).despawn();
    }
}

/// Calculates relative exit weights for vehicles entering from `current_arm_index`.
///
/// Returns an `EntityHashMap`<`u32`> mapping target arm entities to their integer destination weights.
fn calculate_destination_weights(
    arm_blueprints: &[ArmBlueprint],
    current_arm_index: usize,
    roundabout_topology: &RoundaboutTopology,
) -> DestinationWeights {
    /// The minimum destination weight of an exit road. Assigned to U-turns.
    const UTURN_WEIGHT: f32 = 0.05;

    let mut destination_weights = EntityHashMap::default();

    let source_angle = arm_blueprints[current_arm_index].angle().as_radians();

    for (target_index, target_blueprint) in arm_blueprints.iter().enumerate() {
        let target_arm_id = roundabout_topology.get_arm_id_at(target_index);
        let target_angle = target_blueprint.angle().as_radians();

        let difference = target_angle - source_angle;

        // Score based on angle difference of exit arm to entry arm.
        let alignment_score = (1.0 + (difference - PI).cos()) / 2.0;
        let normalized_weight = UTURN_WEIGHT + (1.0 - UTURN_WEIGHT) * alignment_score;

        // Scale to integer range (between UTURN_WEIGHT * 100 and 100).
        let u32_weight = (normalized_weight * 100.0).round() as u32;

        destination_weights.insert(target_arm_id, u32_weight);
    }

    destination_weights
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
