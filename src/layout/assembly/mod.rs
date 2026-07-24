use crate::*;

pub struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, _app: &mut App) {}
}

// The order of with the intra and inter arm sectors are in circulating_sectors
const INTRA_ARM_SECTOR_INDEX: usize = 0;
const INTER_ARM_SECTOR_INDEX: usize = 1;

/// Assembles the roundabout using the blueprint resources.
/// Removes the existing layout and vehicles before calculating and spawning the new layout.
pub fn assemble_roundabout(
    mut commands: Commands,
    existing_vehicles: Query<Entity, (With<Kinematics>, With<Navigator>)>,
    existing_segments: Query<Entity, With<Segment>>,
    existing_spawns: Query<Entity, With<SpawnPoint>>,
    existing_ends: Query<Entity, With<EndPoint>>,
    intersection_blueprint: Res<IntersectionBlueprint>,
    roundabout_circle_blueprint: Res<RoundaboutCircleBlueprint>,
) {
    info!("Assembling roundabout from blueprints");

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

    let arms = &intersection_blueprint.arms;
    // Temporary: I assume that the below line is redundant. Will remove later.
    // sorted_arms.sort_by_cached_key(|arm| std::cmp::Reverse(FloatOrd(arm.angle.as_radians())));
    let number_of_arms = arms.len();

    let segment_entities = SegmentEntities::new(&mut commands, number_of_lanes, number_of_arms);

    for (arm_index, arm) in arms.iter().enumerate() {
        let next_arm_index = if arm_index == 0 {
            number_of_arms - 1
        } else {
            arm_index - 1
        };

        let next_arm_angle = arms[next_arm_index].angle;

        // If the arm has a speed limit override, use that instead of the intersection default speed limit.
        let speed_limit = match arm.speed_limit {
            Some(speed_limit) => speed_limit,
            None => speed_limit,
        };

        for lane_index in 0..number_of_lanes {
            let entry_line_id = segment_entities.entries[arm_index][lane_index];
            let entry_deflection_id = segment_entities.entry_deflections[arm_index][lane_index];
            let exit_line_id = segment_entities.exits[arm_index][lane_index];
            let exit_deflection_id = segment_entities.exit_deflections[next_arm_index][lane_index];

            let intra_arm_sector_id =
                segment_entities.circulating_sectors[arm_index][lane_index][INTRA_ARM_SECTOR_INDEX];
            let inter_arm_sector_id =
                segment_entities.circulating_sectors[arm_index][lane_index][INTER_ARM_SECTOR_INDEX];
            let next_intra_arm_id = segment_entities.circulating_sectors[next_arm_index]
                [lane_index][INTRA_ARM_SECTOR_INDEX];

            let entry_geometry = LaneGeometry::generate(
                LaneType::Entry,
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(entry_deflection_id).insert(Segment::new(
                CubicBezier::new([entry_geometry.deflection_curve]),
                Connection::NextSegments {
                    next_segments: vec![intra_arm_sector_id],
                    requires_yield: true,
                },
                speed_limit,
            ));

            commands.entity(entry_line_id).insert(Segment::new(
                LinearSpline::new(entry_geometry.straight_line),
                Connection::NextSegments {
                    next_segments: vec![entry_deflection_id],
                    requires_yield: false,
                },
                speed_limit,
            ));

            commands.spawn(SpawnPoint {
                segment: entry_line_id,
                max_vehicles_per_second: 0.5,
                destination_weights: EntityHashMap::default(),
            });

            let exit_geometry = LaneGeometry::generate(
                LaneType::Exit,
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            let end_point_id = commands.spawn(EndPoint).id();

            commands.entity(exit_line_id).insert(Segment::new(
                LinearSpline::new(exit_geometry.straight_line),
                Connection::EndPoint {
                    end_point: end_point_id,
                },
                speed_limit,
            ));

            commands.entity(exit_deflection_id).insert(Segment::new(
                CubicBezier::new([exit_geometry.deflection_curve]),
                Connection::NextSegments {
                    next_segments: vec![exit_line_id],
                    requires_yield: false,
                },
                speed_limit,
            ));

            let inter_arm_sector_geometry = SectorGeometry::generate(
                SectorType::InterArm { next_arm_angle },
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(inter_arm_sector_id).insert(Segment::new(
                inter_arm_sector_geometry,
                Connection::NextSegments {
                    next_segments: vec![exit_deflection_id, next_intra_arm_id],
                    requires_yield: false,
                },
                speed_limit,
            ));

            let intra_arm_sector_geometry = SectorGeometry::generate(
                SectorType::IntraArm,
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            commands.entity(intra_arm_sector_id).insert(Segment::new(
                intra_arm_sector_geometry,
                Connection::NextSegments {
                    next_segments: vec![inter_arm_sector_id],
                    requires_yield: false,
                },
                speed_limit,
            ));
        }
    }
}

fn clear_existing_layout(
    commands: &mut Commands,
    existing_vehicles: Query<Entity, (With<Kinematics>, With<Navigator>)>,
    existing_segments: Query<Entity, With<Segment>>,
    existing_spawns: Query<Entity, With<SpawnPoint>>,
    existing_ends: Query<Entity, With<EndPoint>>,
) {
    for vehicle in existing_vehicles {
        commands.entity(vehicle).despawn();
    }

    // Despawn old segments before assembling new layout.
    for entity in existing_segments
        .iter()
        .chain(existing_spawns.iter())
        .chain(existing_ends.iter())
    {
        commands.entity(entity).despawn();
    }
}

/// Stores each segment entity at [arm_index][lane_index].
type SegmentEntity = Vec<Vec<Entity>>;

/// Different Segment entities for different parts of the roundabout.
struct SegmentEntities {
    entries: SegmentEntity,
    entry_deflections: SegmentEntity,
    exits: SegmentEntity,
    exit_deflections: SegmentEntity,
    /// Circulating sectors holds a Vec for intra and inter arms.
    /// Stored as [arm_index][lane_index][intra or inter arm]
    circulating_sectors: Vec<Vec<Vec<Entity>>>,
}

impl SegmentEntities {
    fn new(commands: &mut Commands, number_of_lanes: usize, number_of_arms: usize) -> Self {
        // Create vectors.
        let mut entries = vec![vec![commands.spawn_empty().id(); number_of_lanes]; number_of_arms];
        let mut entry_deflections =
            vec![vec![commands.spawn_empty().id(); number_of_lanes]; number_of_arms];
        let mut exits = vec![vec![commands.spawn_empty().id(); number_of_lanes]; number_of_arms];
        let mut exit_deflections =
            vec![vec![commands.spawn_empty().id(); number_of_lanes]; number_of_arms];
        let mut circulating_sectors =
            vec![vec![vec![commands.spawn_empty().id(); 2]; number_of_lanes]; number_of_arms];

        // Populate vectors with entities.
        for arm_index in 0..number_of_arms {
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

        SegmentEntities {
            entries,
            entry_deflections,
            exits,
            exit_deflections,
            circulating_sectors,
        }
    }
}
