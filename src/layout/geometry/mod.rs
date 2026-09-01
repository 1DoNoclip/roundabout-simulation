//! Calculates the instructions for where to place components to assemble the roundabout layout.

use crate::*;

pub(super) struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, _app: &mut App) {}
}

/// The width of a singular lane of roads and roundabout in metres.
pub(crate) const LANE_WIDTH: f32 = 3.5;

/// Defines the geometry of a singular lane that approaches or exits the roundabout.
/// At least one entry and one exit `LaneGeometry` is required to make up an arm.
pub(crate) struct LaneGeometry {
    /// Straight 100m line road as `[start, end]`.
    straight_line_points: StraightLinePoints,
    /// 4-point `CubicBezier` curve control points as `[start, ..., end]`.
    deflection_curve_points: DeflectionCurvePoints,
}

impl LaneGeometry {
    pub fn generate_entry(
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        LaneGeometry::generate(
            LaneType::Entry,
            arm_angle,
            lane_index,
            roundabout_radius,
            deflection_radius,
        )
    }

    pub fn generate_exit(
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        LaneGeometry::generate(
            LaneType::Exit,
            arm_angle,
            lane_index,
            roundabout_radius,
            deflection_radius,
        )
    }

    pub fn into_curves(self) -> (StraightLinePoints, DeflectionCurvePoints) {
        (self.straight_line_points, self.deflection_curve_points)
    }

    fn generate(
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

        let angular_displacement = deflection_radius / roundabout_radius;

        let arm_vector = Vec3::new(arm_angle.cos, arm_angle.sin, 0.0);
        let perpendicular_vector = Vec3::new(-arm_angle.sin, arm_angle.cos, 0.0);

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
                    straight_line_points: StraightLinePoints([spawn_point_start, deflection_start]),
                    deflection_curve_points: DeflectionCurvePoints([
                        deflection_start,
                        p1,
                        p2,
                        deflection_end,
                    ]),
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
                    straight_line_points: StraightLinePoints([deflection_end, end_point_end]),
                    deflection_curve_points: DeflectionCurvePoints([
                        deflection_start,
                        p1,
                        p2,
                        deflection_end,
                    ]),
                }
            }
        }
    }
}

enum LaneType {
    Entry,
    Exit,
}

pub(crate) struct StraightLinePoints(pub(crate) [Vec3; 2]);

impl CurveLength for StraightLinePoints {
    fn length_metres(&self) -> f32 {
        self.0
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum()
    }
}

impl IntoEvaluators for StraightLinePoints {
    fn into_evaluators(self) -> Evaluators {
        let linear_spline = LinearSpline::new(self.0);
        let curve = linear_spline
            .to_curve()
            .expect("failed to convert LinearSpline into CubicCurve");
        let tangent_curve = curve.clone();

        let position_evaluator = Box::new(move |time| curve.sample_clamped(time));
        let tangent_evaluator =
            Box::new(move |time| tangent_curve.velocity(time).normalize_or_zero());
        // The curvature of the straight line is always 0.0.
        let curvature_evaluator = Box::new(move |_| 0.0);

        Evaluators::new(position_evaluator, tangent_evaluator, curvature_evaluator)
    }
}

pub(crate) struct DeflectionCurvePoints(pub(crate) [Vec3; 4]);

impl CurveLength for DeflectionCurvePoints {
    fn length_metres(&self) -> f32 {
        const TOTAL_SAMPLES: usize = 1_000;

        let cubic_bezier = CubicBezier::new([self.0]);
        match cubic_bezier.to_curve() {
            Ok(curve) => curve
                .iter_positions(TOTAL_SAMPLES)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|pair| pair[0].distance(pair[1]))
                .sum(),
            Err(error) => {
                warn!("failed to convert CubicBezier into CubicCurve: {error}");
                0.0
            }
        }
    }
}

impl IntoEvaluators for DeflectionCurvePoints {
    fn into_evaluators(self) -> Evaluators {
        let cubic_bezier = CubicBezier::new([self.0]);
        let curve = cubic_bezier
            .to_curve()
            .expect("failed to convert CubicBezier into CubicCurve");
        let tangent_curve = curve.clone();
        let curvature_curve = curve.clone();

        let position_evaluator = Box::new(move |time| curve.sample_clamped(time));
        let tangent_evaluator =
            Box::new(move |time| tangent_curve.velocity(time).normalize_or_zero());
        let curvature_evaluator = Box::new(move |time| {
            let d1 = curvature_curve.velocity(time);
            let d2 = curvature_curve.acceleration(time);

            let numerator = (d1.x * d2.y - d1.y * d2.x).abs();
            let denominator = (d1.x.powi(2) + d1.y.powi(2)).powf(1.5);

            if denominator < 1e-6 {
                0.0
            } else {
                numerator / denominator
            }
        });

        Evaluators::new(position_evaluator, tangent_evaluator, curvature_evaluator)
    }
}

/// Defines a singular sector on the circulating part of the roundabout.
pub(crate) struct SectorGeometry {
    /// The radius of the sector.
    radius: f32,
    /// The angle where the sector begins.
    start_angle: f32,
    /// The angle where the sector ends.
    end_angle: f32,
}

impl SectorGeometry {
    pub fn generate_intra_arm(
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        SectorGeometry::generate(
            SectorType::IntraArm,
            arm_angle,
            lane_index,
            roundabout_radius,
            deflection_radius,
        )
    }

    pub fn generate_inter_arm(
        arm_angle: Rot2,
        next_arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        SectorGeometry::generate(
            SectorType::InterArm { next_arm_angle },
            arm_angle,
            lane_index,
            roundabout_radius,
            deflection_radius,
        )
    }

    fn generate(
        sector_type: SectorType,
        arm_angle: Rot2,
        lane_index: usize,
        roundabout_radius: f32,
        deflection_radius: f32,
    ) -> Self {
        // The radius of this circulating sector.
        let radius = roundabout_radius + (lane_index as f32 * LANE_WIDTH);
        let angular_displacement = deflection_radius / roundabout_radius;

        let (start_angle, raw_end_angle) = match sector_type {
            SectorType::IntraArm => {
                let start = arm_angle.as_radians() + angular_displacement;
                let end = arm_angle.as_radians() - angular_displacement;
                (start, end)
            }
            SectorType::InterArm { next_arm_angle } => {
                let start = arm_angle.as_radians() - angular_displacement;
                let end = next_arm_angle.as_radians() + angular_displacement;
                (start, end)
            }
        };

        let clockwise_sweep = (start_angle - raw_end_angle).rem_euclid(std::f32::consts::TAU);
        // Ensures that end_angle is less than start_angle.
        let end_angle = start_angle - clockwise_sweep;

        Self {
            radius,
            start_angle,
            end_angle,
        }
    }
}

impl CurveLength for SectorGeometry {
    fn length_metres(&self) -> f32 {
        self.radius * (self.start_angle - self.end_angle)
    }
}

impl IntoEvaluators for SectorGeometry {
    fn into_evaluators(self) -> Evaluators {
        let start_angle = self.start_angle;
        let delta_angle = self.end_angle - self.start_angle;
        let radius = self.radius;

        let position_evaluator = Box::new(move |time| {
            let angle: f32 = start_angle + time * delta_angle;
            Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
        });

        let tangent_evaluator = Box::new(move |time| {
            let angle: f32 = start_angle + time * delta_angle;
            // The tangent derivative of (cos(theta), sin(theta)) is (-sin(theta), cos(theta)).
            // We scale by signum of delta_angle to preserve direction (clockwise or anti-clockwise).
            (Vec3::new(-angle.sin(), angle.cos(), 0.0) * delta_angle.signum()).normalize_or_zero()
        });

        let curvature_evaluator = Box::new(move |_| if radius > 1e-6 { 1.0 / radius } else { 0.0 });

        Evaluators::new(position_evaluator, tangent_evaluator, curvature_evaluator)
    }
}

/// Decides where on the circle the sector lies.
enum SectorType {
    /// Between Arm N's entry and Arm (N + 1)'s exit.
    IntraArm,
    /// Between Arm N's exit and Arm N's entry.
    InterArm { next_arm_angle: Rot2 },
}
