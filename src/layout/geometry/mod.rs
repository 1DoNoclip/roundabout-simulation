use crate::*;

pub struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, _app: &mut App) {}
}

pub enum LaneGeometryType {
    Entry,
    Exit,
}

/// Defines the geometry of a singular approach lane.
pub struct LaneGeometry {
    /// Straight 100m line road as [start, end].
    pub straight_line: [Vec3; 2],
    /// 4-point Cubic Bezier curve control points as [start, ..., end].
    pub deflection_curve: [Vec3; 4],
}

impl LaneGeometry {
    pub fn generate(
        geometry_type: LaneGeometryType,
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        let lane_offset = (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);
        let target_ring_radius =
            roundabout_radius + (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);
        let deflection_start_distance = roundabout_radius + deflection_radius;

        let arm_vector = Vec3::new(arm_angle.cos, 0.0, arm_angle.sin);
        let perpendicular_vector = Vec3::new(-arm_angle.sin, 0.0, arm_angle.cos);

        let angular_displacement = deflection_radius / roundabout_radius;
        let handle_strength = deflection_radius * 0.35;

        match geometry_type {
            LaneGeometryType::Entry => {
                // Entry sits on the left side of the arm centerline (+perpendicular).
                let deflection_start =
                    (arm_vector * deflection_start_distance) + (perpendicular_vector * lane_offset);
                let approach_start = deflection_start + (arm_vector * 100.0);

                let entry_angle = arm_angle * Rot2::radians(-angular_displacement);
                let deflection_end = Vec3::new(
                    target_ring_radius * entry_angle.cos,
                    0.0,
                    target_ring_radius * entry_angle.sin,
                );

                let roundabout_tangent = Vec3::new(-entry_angle.sin, 0.0, entry_angle.cos);

                let p1 = deflection_start - (arm_vector * handle_strength);
                let p2 = deflection_end + (roundabout_tangent * handle_strength);

                LaneGeometry {
                    straight_line: [approach_start, deflection_start],
                    deflection_curve: [deflection_start, p1, p2, deflection_end],
                }
            }
            LaneGeometryType::Exit => {
                // Exit sits on the right side of the arm centerline (-perpendicular).
                let deflection_end_point =
                    (arm_vector * deflection_start_distance) - (perpendicular_vector * lane_offset);
                let exit_end_point = deflection_end_point + (arm_vector * 100.0);

                // Exit connects to the ring slightly before the arm angle (+angular displacement).
                let exit_angle = arm_angle * Rot2::radians(angular_displacement);
                let deflection_start_on_ring = Vec3::new(
                    target_ring_radius * exit_angle.cos,
                    0.0,
                    target_ring_radius * exit_angle.sin,
                );

                // Tangent pointing out of the roundabout ring.
                let exit_tangent = Vec3::new(-exit_angle.sin, 0.0, exit_angle.cos);

                let p1 = deflection_start_on_ring + (exit_tangent * handle_strength);
                let p2 = deflection_end_point - (arm_vector * handle_strength);

                LaneGeometry {
                    // From ring out to exit straight;
                    straight_line: [deflection_end_point, exit_end_point],
                    deflection_curve: [deflection_start_on_ring, p1, p2, deflection_end_point],
                }
            }
        }
    }
}
