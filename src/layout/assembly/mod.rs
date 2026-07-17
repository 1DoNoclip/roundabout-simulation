use crate::*;
use bevy::math::FloatOrd;
use enterpolation::bezier::Bezier;

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

    let mut end_points = vec![];

    for (arm_index, arm) in sorted_arms.iter().enumerate() {
        for lane_index in 0..number_of_lanes {
            let entry_geometry = LaneGeometry::build(
                LaneGeometryType::Entry,
                arm.angle,
                lane_index,
                inner_radius,
                deflection_radius,
            );

            let entry_line = Linear::builder()
                .elements(vec![
                    entry_geometry.lane_approach[0],
                    entry_geometry.lane_approach[1],
                ])
                .knots(vec![0.0, 1.0])
                .build()
                .expect("failed to build linear entry path");

            let entry_line_entity = commands
                .spawn((Segment::new(entry_line, speed_limit),))
                .id();

            let entry_control_points = entry_geometry
                .deflection_curve
                .iter()
                .map(|vector| vector.to_array())
                .collect::<Vec<_>>();

            let entry_deflection = Bezier::builder()
                .elements(entry_geometry.deflection_curve.to_vec())
                .domain(0.0, 1.0)
                .build()
                .expect("failed to build cubic deflection spline")
                .map(Vec3::from);
        }
    }
}
