use crate::*;

pub struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, _app: &mut App) {}
}

/// The width of a singular lane of roads and roundabout in metres.
pub const LANE_WIDTH: f32 = 3.5;

pub enum LaneType {
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
        geometry_type: LaneType,
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        // The radius to the centre of the target circulating lane.
        let target_ring_radius = roundabout_radius + (lane_index as f32 * LANE_WIDTH);
        // The offset of the lane from the inner lane.
        let lane_offset = (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);
        let deflection_start_distance = roundabout_radius + deflection_radius;

        let arm_vector = Vec3::new(arm_angle.cos, arm_angle.sin, 0.0);
        let perpendicular_vector = Vec3::new(-arm_angle.sin, arm_angle.cos, 0.0);

        let angular_displacement = deflection_radius / roundabout_radius;

        match geometry_type {
            LaneType::Entry => {
                // Entry sits on the left side of the arm centerline (-perpendicular).
                let deflection_start =
                    (arm_vector * deflection_start_distance) - (perpendicular_vector * lane_offset);

                // Entry starts 100m out and travels in towards deflection_start.
                let spawn_point_start = deflection_start + (arm_vector * 100.0);

                // Entry joins the ring slightly before the arm angle (angular_displacement).
                let entry_angle = arm_angle * Rot2::radians(-angular_displacement);
                let deflection_end = Vec3::new(
                    target_ring_radius * entry_angle.cos,
                    target_ring_radius * entry_angle.sin,
                    0.0,
                );

                let chord_length = (deflection_end - deflection_start).length();
                let handle_length = chord_length / 3.0;

                let clockwise_tangent = Vec3::new(entry_angle.sin, -entry_angle.cos, 0.0);

                // Control points push inwards (-arm_vector) then along the clockwise ring tangent.
                let p1 = deflection_start - (arm_vector * handle_length);
                let p2 = deflection_end - (clockwise_tangent * handle_length);

                LaneGeometry {
                    straight_line: [spawn_point_start, deflection_start],
                    deflection_curve: [deflection_start, p1, p2, deflection_end],
                }
            }
            LaneType::Exit => {
                // Exit sits on the right side of the arm centerline (+perpendicular).
                let deflection_end =
                    (arm_vector * deflection_start_distance) + (perpendicular_vector * lane_offset);

                // Exit straight travels from deflection end outwards (+arm_vector).
                let end_point_end = deflection_end + (arm_vector * 100.0);

                // Exit leaves the ring slightly after the arm angle (angular_displacement).
                let exit_angle = arm_angle * Rot2::radians(angular_displacement);
                let deflection_start = Vec3::new(
                    target_ring_radius * exit_angle.cos,
                    target_ring_radius * exit_angle.sin,
                    0.0,
                );

                let chord_length = (deflection_end - deflection_start).length();
                let handle_length = chord_length / 3.0;

                let clockwise_tangent = Vec3::new(exit_angle.sin, -exit_angle.cos, 0.0);

                // Control points leave along ring tangent, then align outwards (+arm_vector).
                let p1 = deflection_start + (clockwise_tangent * handle_length);
                let p2 = deflection_end - (arm_vector * handle_length);

                LaneGeometry {
                    straight_line: [deflection_end, end_point_end],
                    deflection_curve: [deflection_start, p1, p2, deflection_end],
                }
            }
        }
    }
}

pub enum SectorType {
    /// Between Arm N's exit and Arm N's entry.
    InterArm { next_arm_angle: Rot2 },
    /// Between Arm N's entry and Arm (N + 1)'s exit.
    IntraArm,
}

pub struct CirculatingSectorGeometry {
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
}

impl CirculatingSectorGeometry {
    pub fn generate(
        sector_type: SectorType,
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        let radius = roundabout_radius + (lane_index as f32 * LANE_WIDTH);
        let angular_displacement = deflection_radius / radius;

        let (start_angle, raw_end_angle) = match sector_type {
            SectorType::InterArm { next_arm_angle } => {
                let start = arm_angle.as_radians() + angular_displacement;
                let end = next_arm_angle.as_radians() - angular_displacement;
                (start, end)
            }
            SectorType::IntraArm => {
                let start = arm_angle.as_radians() - angular_displacement;
                let end = arm_angle.as_radians() + angular_displacement;
                (start, end)
            }
        };

        let clockwise_sweep = (start_angle - raw_end_angle).rem_euclid(std::f32::consts::TAU);
        let end_angle = start_angle - clockwise_sweep;

        Self {
            radius,
            start_angle,
            end_angle,
        }
    }
}

impl CurveLength for CirculatingSectorGeometry {
    fn length(&self) -> f32 {
        // Todo: Improve this code, should not panic.
        // Enforce constraints in construction of CirculatingSectorGeometry
        // to prevent end_angle bigger than start_angle.
        if self.end_angle > self.start_angle {
            panic!(
                "end_angle ({}) should not be greater than start angle ({})",
                self.end_angle, self.start_angle
            );
        }
        self.radius * (self.start_angle - self.end_angle).abs()
    }
}

impl IntoEvaluator for CirculatingSectorGeometry {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static> {
        Box::new(move |time| {
            let angle = self.start_angle + time * (self.end_angle - self.start_angle);
            Vec3::new(self.radius * angle.cos(), self.radius * angle.sin(), 0.0)
        })
    }
}
