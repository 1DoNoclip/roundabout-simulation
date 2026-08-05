use crate::*;
use bevy::math::FloatOrd;
use std::f32::consts::PI;

pub struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, _app: &mut App) {}
}

// The order of with the intra and inter arm sectors are in circulating_sectors
const INTRA_ARM_SECTOR_INDEX: usize = 0;
const INTER_ARM_SECTOR_INDEX: usize = 1;

/// Assembles the roundabout using the blueprint resources.
/// Removes the existing layout and vehicles before spawning the new layout.
pub fn assemble_roundabout(
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

    let number_of_lanes = intersection_blueprint.number_of_lanes;
    let inner_radius = roundabout_circle_blueprint.radius;
    let deflection_radius = intersection_blueprint.deflection_radius;
    let speed_limit = intersection_blueprint.speed_limit;
    let arm_blueprints = &intersection_blueprint.arms;
    let number_of_arms = arm_blueprints.len();

    let roundabout_topology =
        RoundaboutTopology::new(&mut commands, number_of_lanes, number_of_arms);

    let mut sorted_arms = arm_blueprints.clone();
    sorted_arms.sort_by_cached_key(|arm| std::cmp::Reverse(FloatOrd(arm.angle.as_radians())));
    let sorted_arms = sorted_arms;
    for (arm_index, arm_blueprint) in sorted_arms.iter().enumerate() {
        let next_arm_index = (arm_index + 1) % number_of_arms;
        let next_arm_angle = sorted_arms[next_arm_index].angle;

        let arm_id = roundabout_topology.get_arm_at(arm_index);
        // Add the Arm component.
        commands.entity(arm_id).insert((
            Name::new(format!("Arm [{arm_index}]")),
            Arm::new(
                arm_index,
                arm_blueprint.angle,
                arm_blueprint.max_vehicles_per_second,
                calculate_destination_weights(&sorted_arms, arm_index, &roundabout_topology),
            ),
        ));

        // If the arm has a speed limit override, use that instead of the intersection default speed limit.
        let speed_limit = match arm_blueprint.speed_limit_override {
            Some(speed_limit) => speed_limit,
            None => speed_limit,
        };

        for lane_index in 0..number_of_lanes {
            // A unique identifier for naming purposes.
            // Serves no functional purpose, other than being
            // able to identify related Entities in the inspector.
            let unique_identifier = format!("[{arm_index}, {lane_index}]");
            let entities =
                roundabout_topology.get_entities_for(arm_index, lane_index, next_arm_index);

            let entry_geometry = LaneGeometry::generate(
                LaneType::Entry,
                arm_blueprint.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(entities.entry_deflection).insert((
                Name::new(format!("EntryDeflection {unique_identifier}")),
                Segment::new(
                    CubicBezier::new([entry_geometry.deflection_curve]),
                    Connection::Merge {
                        next_segment_id: entities.inter_arm_sector,
                    },
                    speed_limit,
                ),
            ));

            commands.entity(entities.entry_line).insert((
                Name::new(format!("EntryLine {unique_identifier}")),
                Segment::new(
                    LinearSpline::new(entry_geometry.straight_line),
                    Connection::Direct {
                        next_segment_id: entities.entry_deflection,
                    },
                    speed_limit,
                ),
            ));

            commands.spawn((
                Name::new(format!("SpawnPoint {unique_identifier}")),
                SpawnPoint::new(arm_id, lane_index, entities.entry_line),
            ));

            let exit_geometry = LaneGeometry::generate(
                LaneType::Exit,
                arm_blueprint.angle,
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

            commands.entity(entities.exit_line).insert((
                Name::new(format!("ExitLine {unique_identifier}")),
                Segment::new(
                    LinearSpline::new(exit_geometry.straight_line),
                    Connection::EndPoint { end_point_id },
                    speed_limit,
                ),
            ));

            commands.entity(entities.exit_deflection).insert((
                Name::new(format!("ExitDeflection {unique_identifier}")),
                Segment::new(
                    CubicBezier::new([exit_geometry.deflection_curve]),
                    Connection::Direct {
                        next_segment_id: entities.exit_line,
                    },
                    speed_limit,
                ),
            ));

            let intra_arm_sector_geometry = SectorGeometry::generate(
                SectorType::IntraArm,
                arm_blueprint.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(entities.intra_arm_sector).insert((
                Name::new(format!("IntraArmSector {unique_identifier}")),
                Segment::new(
                    intra_arm_sector_geometry,
                    Connection::Direct {
                        next_segment_id: entities.inter_arm_sector,
                    },
                    speed_limit,
                ),
            ));

            let inter_arm_sector_geometry = SectorGeometry::generate(
                SectorType::InterArm { next_arm_angle },
                arm_blueprint.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(entities.inter_arm_sector).insert((
                Name::new(format!("InterArmSector {unique_identifier}")),
                Segment::new(
                    inter_arm_sector_geometry,
                    Connection::Diverge {
                        exit_arm_index: next_arm_index,
                        exit_segment_id: entities.next_exit_deflection,
                        circulating_segment_id: entities.next_intra_arm_sector,
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
    info!("Despawning vehicles");
    for vehicle in existing_vehicles {
        commands.entity(vehicle).despawn();
    }

    // Despawn old segments before assembling new layout.
    for entity in existing_segments
        .iter()
        .chain(existing_spawns.iter())
        .chain(existing_ends.iter())
    {
        info!("Despawning segment entity");
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

    let source_angle = arm_blueprints[current_arm_index].angle.as_radians();

    for (target_index, target_blueprint) in arm_blueprints.iter().enumerate() {
        let target_arm_id = roundabout_topology.get_arm_at(target_index);
        let target_angle = target_blueprint.angle.as_radians();

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

/// Stores each segment entity at [arm_index][lane_index].
type SegmentMatrix = Vec<Vec<Entity>>;

/// Different Segment matrices for different parts of the roundabout.
struct RoundaboutTopology {
    arms: Vec<Entity>,
    entries: SegmentMatrix,
    entry_deflections: SegmentMatrix,
    exits: SegmentMatrix,
    exit_deflections: SegmentMatrix,
    /// Circulating sectors holds a Vec for intra and inter arms.
    /// Stored as [arm_index][lane_index][intra or inter arm]
    circulating_sectors: Vec<Vec<Vec<Entity>>>,
}

impl RoundaboutTopology {
    fn new(commands: &mut Commands, number_of_lanes: usize, number_of_arms: usize) -> Self {
        // Create vectors.
        let mut arms = vec![Entity::PLACEHOLDER; number_of_arms];
        let mut entries = vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
        let mut entry_deflections =
            vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
        let mut exits = vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
        let mut exit_deflections = vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
        let mut circulating_sectors =
            vec![vec![vec![Entity::PLACEHOLDER; 2]; number_of_lanes]; number_of_arms];

        // Populate vectors with entities.
        for arm_index in 0..number_of_arms {
            arms[arm_index] = commands.spawn_empty().id();
            for lane_index in 0..number_of_lanes {
                entries[arm_index][lane_index] = commands.spawn_empty().id();
                entry_deflections[arm_index][lane_index] = commands.spawn_empty().id();
                exits[arm_index][lane_index] = commands.spawn_empty().id();
                exit_deflections[arm_index][lane_index] = commands.spawn_empty().id();
                circulating_sectors[arm_index][lane_index][INTRA_ARM_SECTOR_INDEX] =
                    commands.spawn_empty().id();
                circulating_sectors[arm_index][lane_index][INTER_ARM_SECTOR_INDEX] =
                    commands.spawn_empty().id();
            }
        }

        RoundaboutTopology {
            arms,
            entries,
            entry_deflections,
            exits,
            exit_deflections,
            circulating_sectors,
        }
    }

    fn get_arm_at(&self, arm_index: usize) -> Entity {
        self.arms[arm_index]
    }

    /// Get the entities for the current iteration of assembly.
    fn get_entities_for(
        &self,
        arm_index: usize,
        lane_index: usize,
        next_arm_index: usize,
    ) -> CurrentIterationEntities {
        CurrentIterationEntities {
            entry_line: self.entries[arm_index][lane_index],
            entry_deflection: self.entry_deflections[arm_index][lane_index],
            exit_line: self.exits[arm_index][lane_index],
            exit_deflection: self.exit_deflections[arm_index][lane_index],
            intra_arm_sector: self.circulating_sectors[arm_index][lane_index]
                [INTRA_ARM_SECTOR_INDEX],
            inter_arm_sector: self.circulating_sectors[arm_index][lane_index]
                [INTER_ARM_SECTOR_INDEX],
            next_intra_arm_sector: self.circulating_sectors[next_arm_index][lane_index]
                [INTRA_ARM_SECTOR_INDEX],
            next_exit_deflection: self.exit_deflections[next_arm_index][lane_index],
        }
    }
}

struct CurrentIterationEntities {
    entry_line: Entity,
    entry_deflection: Entity,
    exit_line: Entity,
    exit_deflection: Entity,
    intra_arm_sector: Entity,
    inter_arm_sector: Entity,
    next_intra_arm_sector: Entity,
    next_exit_deflection: Entity,
}
