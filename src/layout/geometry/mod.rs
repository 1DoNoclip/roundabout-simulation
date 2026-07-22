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
        let lane_offset = (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);
        let target_ring_radius =
            roundabout_radius + (LANE_WIDTH / 2.0) + (lane_index as f32 * LANE_WIDTH);
        let deflection_start_distance = roundabout_radius + deflection_radius;

        let arm_vector = Vec3::new(arm_angle.cos, arm_angle.sin, 0.0);
        let perpendicular_vector = Vec3::new(-arm_angle.sin, arm_angle.cos, 0.0);

        let angular_displacement = deflection_radius / roundabout_radius;
        let handle_strength = deflection_radius * 0.35;

        match geometry_type {
            LaneType::Entry => {
                // Entry sits on the left side of the arm centerline (+perpendicular).
                let deflection_start =
                    (arm_vector * deflection_start_distance) + (perpendicular_vector * lane_offset);

                // Entry starts 100m out and travels in towards deflection_start.
                let spawn_point_start = deflection_start + (arm_vector * 100.0);

                // For clockwise flow, entry connects slightly ahead anti-clockwise (+angular_displacement)
                // so traffic merges smoothly clockwise into the ring.
                let entry_angle = arm_angle * Rot2::radians(angular_displacement);
                let deflection_end = Vec3::new(
                    target_ring_radius * entry_angle.cos,
                    target_ring_radius * entry_angle.sin,
                    0.0,
                );

                let clockwise_tangent = Vec3::new(entry_angle.sin, -entry_angle.cos, 0.0);

                // Control points push inwards (-arm_vector) then along the clockwise ring tangent.
                let p1 = deflection_start - (arm_vector * handle_strength);
                let p2 = deflection_end - (clockwise_tangent * handle_strength);

                LaneGeometry {
                    straight_line: [spawn_point_start, deflection_start],
                    deflection_curve: [deflection_start, p1, p2, deflection_end],
                }
            }
            LaneType::Exit => {
                // Exit sits on the right side of the arm centerline (-perpendicular).
                let deflection_end =
                    (arm_vector * deflection_start_distance) - (perpendicular_vector * lane_offset);

                // Exit straight travels from deflection end outwards (+arm_vector).
                let end_point_end = deflection_end + (arm_vector * 100.0);

                // Exit leaves the ring slightly before reaching the arm angle (-angular_displacement).
                let exit_angle = arm_angle * Rot2::radians(-angular_displacement);
                let deflection_start = Vec3::new(
                    target_ring_radius * exit_angle.cos,
                    target_ring_radius * exit_angle.sin,
                    0.0,
                );

                let clockwise_tangent = Vec3::new(exit_angle.sin, -exit_angle.cos, 0.0);

                // Control points leave along ring tangent, then align outwards (+arm_vector).
                let p1 = deflection_start + (clockwise_tangent * handle_strength);
                let p2 = deflection_end - (arm_vector * handle_strength);

                LaneGeometry {
                    straight_line: [deflection_end, end_point_end],
                    deflection_curve: [deflection_start, p1, p2, deflection_end],
                }
            }
        }
    }
}

pub enum SectorType {
    InterArm,
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
        next_arm_angle: Option<Rot2>,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        let radius = roundabout_radius + (lane_index as f32 * LANE_WIDTH);

        let angular_displacement = deflection_radius / radius;

        let (start_angle, end_angle) = match sector_type {
            SectorType::InterArm => {
                let start = arm_angle.as_radians() - angular_displacement;
                let end = next_arm_angle.unwrap().as_radians() + angular_displacement;
                (start, end)
            }
            SectorType::IntraArm => {
                let start = arm_angle.as_radians() + angular_displacement;
                let end = arm_angle.as_radians() - angular_displacement;
                (start, end)
            }
        };

        Self {
            radius,
            start_angle,
            end_angle,
        }
    }
}

impl CurveLength for CirculatingSectorGeometry {
    fn length(&self) -> f32 {
        let delta_angle = (self.end_angle - self.start_angle).abs();
        self.radius * delta_angle
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
