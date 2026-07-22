use crate::*;
use bevy::math::FloatOrd;

pub struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            // Only runs if the blueprint resources have changed since last frame.
            assemble_roundabout.run_if(
                // Uses `.or_eager` instead of `.or_else` so both change ticks are checked
                // and cleared on frame 1. Lazy evaluation (`.or_else`) short-circuits,
                // leaving the second blueprint's tick 'unread' and triggering a redundant
                // second assembly run on frame 2.
                // The simulation should be able to handle redundant roundabout rebuilds without issues,
                // but redundant rebuilds do result in unneccessary work to be carried out.
                resource_changed::<IntersectionBlueprint>
                    .or_eager(resource_changed::<RoundaboutCircleBlueprint>),
            ),
        );
    }
}

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

    const INTRA_ARM_SECTOR_INDEX: usize = 0;
    const INTER_ARM_SECTOR_INDEX: usize = 1;

    let number_of_lanes = intersection_blueprint.number_of_lanes;
    let inner_radius = roundabout_circle_blueprint.radius;
    let deflection_radius = intersection_blueprint.deflection_radius;
    let speed_limit = intersection_blueprint.speed_limit;

    let mut sorted_arms = intersection_blueprint.arms.clone();
    sorted_arms.sort_by_key(|arm| FloatOrd(arm.angle.as_radians()));
    let sorted_arms = sorted_arms;
    let number_of_arms = sorted_arms.len();

    let mut arm_entries = vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
    let mut arm_entry_deflections =
        vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
    let mut arm_exits = vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
    let mut arm_exit_deflections = vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms];
    let mut circulating_sectors =
        vec![vec![vec![Entity::PLACEHOLDER; number_of_lanes]; 2]; number_of_arms];

    for arm_index in 0..number_of_arms {
        for lane_index in 0..number_of_lanes {
            arm_entries[arm_index][lane_index] = commands.spawn_empty().id();
            arm_entry_deflections[arm_index][lane_index] = commands.spawn_empty().id();
            arm_exits[arm_index][lane_index] = commands.spawn_empty().id();
            arm_exit_deflections[arm_index][lane_index] = commands.spawn_empty().id();
            circulating_sectors[arm_index][INTRA_ARM_SECTOR_INDEX][lane_index] =
                commands.spawn_empty().id();
            circulating_sectors[arm_index][INTER_ARM_SECTOR_INDEX][lane_index] =
                commands.spawn_empty().id();
        }
    }

    for (arm_index, arm) in sorted_arms.iter().enumerate() {
        let next_arm_index = if arm_index == 0 {
            number_of_arms - 1
        } else {
            arm_index - 1
        };

        let next_arm_angle = sorted_arms[next_arm_index].angle;

        for lane_index in 0..number_of_lanes {
            let entry_deflection_id = arm_entry_deflections[arm_index][lane_index];
            let entry_line_id = arm_entries[arm_index][lane_index];
            let exit_line_id = arm_exits[arm_index][lane_index];
            let exit_deflection_id = arm_exit_deflections[next_arm_index][lane_index];

            let intra_arm_sector_id =
                circulating_sectors[arm_index][INTRA_ARM_SECTOR_INDEX][lane_index];
            let inter_arm_sector_id =
                circulating_sectors[arm_index][INTER_ARM_SECTOR_INDEX][lane_index];
            let next_intra_arm_id =
                circulating_sectors[next_arm_index][INTRA_ARM_SECTOR_INDEX][lane_index];

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

            let inter_arm_sector_geometry = CirculatingSectorGeometry::generate(
                SectorType::InterArm,
                arm.angle,
                Some(next_arm_angle),
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

            let intra_arm_sector_geometry = CirculatingSectorGeometry::generate(
                SectorType::IntraArm,
                arm.angle,
                None,
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
