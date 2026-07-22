use crate::*;
use bevy::math::FloatOrd;

pub struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn assemble_roundabout(
    mut commands: Commands,
    intersection_blueprint: Res<IntersectionBlueprint>,
    roundabout_circle_blueprint: Res<RoundaboutCircleBlueprint>,
) {
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
        vec![vec![Entity::PLACEHOLDER; number_of_lanes]; number_of_arms * 2];

    // Spawn segments, spawn points and end points.
    for (arm_index, arm) in sorted_arms.iter().enumerate() {
        for lane_index in 0..number_of_lanes {
            // The geometry for the entry lane.
            let entry_geometry = LaneGeometry::generate(
                LaneType::Entry,
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            // The straight entry road.
            let entry_line_id = commands
                .spawn(Segment::new(
                    LinearSpline::new(entry_geometry.straight_line),
                    Connection::NextSegments {
                        next_segments: vec![],
                        requires_yield: false,
                    },
                    speed_limit,
                ))
                .id();
            arm_entries[arm_index][lane_index] = entry_line_id;

            commands.spawn(SpawnPoint {
                segment: entry_line_id,
                max_vehicles_per_second: 0.5,
                destination_weights: EntityHashMap::default(),
            });

            // Entry deflection curve.
            let entry_deflection_id = commands
                .spawn(Segment::new(
                    CubicBezier::new([entry_geometry.deflection_curve]),
                    Connection::NextSegments {
                        next_segments: vec![],
                        requires_yield: false,
                    },
                    speed_limit,
                ))
                .id();
            arm_entry_deflections[arm_index][lane_index] = entry_deflection_id;

            // The geometry for the exit lane.
            let exit_geometry = LaneGeometry::generate(
                LaneType::Exit,
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            let end_point_id = commands.spawn(EndPoint).id();

            // The straight exit road.
            let exit_line_id = commands
                .spawn(Segment::new(
                    LinearSpline::new(exit_geometry.straight_line),
                    Connection::EndPoint {
                        end_point: end_point_id,
                    },
                    speed_limit,
                ))
                .id();
            arm_exits[arm_index][lane_index] = exit_line_id;

            let exit_deflection_id = commands
                .spawn(Segment::new(
                    CubicBezier::new([exit_geometry.deflection_curve]),
                    Connection::NextSegments {
                        next_segments: vec![],
                        requires_yield: false,
                    },
                    speed_limit,
                ))
                .id();
            arm_exit_deflections[arm_index][lane_index] = exit_deflection_id;

            // Circulating sectors.
            // Inter-arm (Entry N -> Exit N + 1).
            let sector_a_index = arm_index * 2;
            // Weave (Exit N -> Entry N).
            let sector_b_index = arm_index * 2 + 1;

            let sector_a_geometry
        }
    }
}
