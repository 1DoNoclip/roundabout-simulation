use crate::*;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_layout, draw_vehicles).chain());
    }
}

pub fn draw_vehicles(mut gizmos: Gizmos, vehicles: Query<&Transform, With<Navigator>>) {
    for transform in vehicles.iter() {
        // Draws a bright cyan circle with a 1.0 pixel radius
        // at the vehicle's current coordinates.
        gizmos.circle_2d(
            transform.translation.truncate(),
            1.0,
            Color::linear_rgb(255.0, 100.0, 0.0),
        );
    }
}

pub fn draw_layout(mut gizmos: Gizmos, segments: Query<&Segment>) {
    const NUMBER_OF_SAMPLES: usize = 100;

    for segment in segments {
        let gizmo_colors = GizmoColors::get_colors(&segment.connection);

        let mut previous_point = (segment.evaluator)(0.0);
        for step in 1..=NUMBER_OF_SAMPLES {
            let time = step as f32 / NUMBER_OF_SAMPLES as f32;
            let current_point = (segment.evaluator)(time);
            gizmos.line(previous_point, current_point, gizmo_colors.segment);
            previous_point = current_point;
        }

        // Small sphere marker at the segment end point.
        gizmos.sphere(Isometry3d::from_translation(previous_point), 0.75, gizmo_colors.point);
    }
}

struct GizmoColors {
    /// The color of the segment.
    segment: Color,
    /// The color of the end of the segment (a point placed at the end).
    point: Color,
}

impl GizmoColors {
    fn srgb_u8(segment: [u8; 3], point: [u8; 3]) -> Self {
        GizmoColors {
            segment: Color::srgb_u8(segment[0], segment[1], segment[2]),
            point: Color::srgb_u8(point[0], point[1], point[2]),
        }
    }

    /// Uses a segment's connection type to determine the color of the segment and the end of the segment.
    fn get_colors(connection: &Connection) -> GizmoColors {
        match connection {
            Connection::NextSegments {
                requires_yield: true,
                ..
            } => {
                // Yellow / dark yellow.
                GizmoColors::srgb_u8([200, 200, 46], [149, 149, 34])
            }
            Connection::NextSegments {
                requires_yield: false,
                ..
            } => {
                // White / grey.
                GizmoColors::srgb_u8([203, 203, 203], [142, 142, 142])
            }
            Connection::EndPoint { .. } => {
                // red / dark.
                GizmoColors::srgb_u8([200, 20, 32], [161, 16, 25])
            }
        }
    }
}
