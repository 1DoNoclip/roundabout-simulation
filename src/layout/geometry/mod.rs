use crate::*;
use std::f32::consts::PI;

pub struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {}
}

/// Defines the geometry of a singular approach lane.
pub struct LaneGeometry {
    /// Straight 100m approach road as [start, end].
    pub lane_approach: [Vec3; 2],
    /// 4-point Cubic Bezier curve control points.
    pub deflection_curve: [Vec3; 4],
}

impl LaneGeometry {
    pub fn build(
        arm_angle_degrees: f32,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        let arm_angle_radians = arm_angle_degrees * (PI / 180.0);

        // Orientation.
        // Points along the road's axis from the centre of the roundabout.
        let arm_vector = Vec3::new(arm_angle_radians.cos(), 0.0, arm_angle_radians.sin());
        let perpendicular_vector =
            Vec3::new(-arm_angle_radians.sin(), 0.0, arm_angle_radians.cos());
        let lane_offset = (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);
        let target_ring_radius =
            roundabout_radius + (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);

        // Deflection start point.
        let deflection_start_distance = roundabout_radius + deflection_radius;
        // p0, where the approach road finishes and the deflection curve begins.
        // Represents point between end of approach and start of deflection.
        let deflection_start =
            (arm_vector * deflection_start_distance) + (perpendicular_vector * lane_offset);

        // Approach road.
        // Approach starts 100m from beginning of deflection curve.
        let approach_start = deflection_start + (arm_vector * 100.0);
        let straight_approach = [approach_start, deflection_start];

        // Deflection spline points.
        let angular_displacement = deflection_radius / roundabout_radius;
        let entry_angle = arm_angle_radians - angular_displacement;

        // p3, where the deflection merges onto the roundabout.
        let deflection_end = Vec3::new(
            target_ring_radius * entry_angle.cos(),
            0.0,
            target_ring_radius * entry_angle.sin(),
        );

        let roundabout_tangent = Vec3::new(-entry_angle.sin(), 0.0, entry_angle.cos());

        // Easing handles.
        let handle_strength = deflection_radius * 0.35;
        let p1 = deflection_start - (arm_vector * handle_strength);
        let p2 = deflection_end + (roundabout_tangent * handle_strength);

        LaneGeometry {
            lane_approach: straight_approach,
            deflection_curve: [deflection_start, p1, p2, deflection_end],
        }
    }
}
