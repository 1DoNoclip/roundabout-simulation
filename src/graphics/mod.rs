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
            Color::linear_rgb(255.0, 0.0, 0.0),
        );
    }
}

pub fn draw_layout(mut gizmos: Gizmos, segments: Query<&Segment>) {
    const SAMPLE_STEPS: usize = 50;

    for segment in &segments {
        let color = match segment.connection {
            Connection::NextSegments {
                requires_yield: true,
                ..
            } => Color::srgb(1.0, 0.2, 0.2), // Red for yield.
            Connection::NextSegments {
                requires_yield: false,
                ..
            } => Color::srgb(0.2, 0.8, 1.0), // Blue for standard continuation.
            Connection::EndPoint { .. } => Color::srgb(0.2, 1.0, 0.2), // Green for exit.
        };

        // Draw the segment's path
        let mut previous_point = (segment.evaluator)(0.0);
        for step in 1..=SAMPLE_STEPS {
            let time = step as f32 / SAMPLE_STEPS as f32;
            let current_point = (segment.evaluator)(time);

            gizmos.line(previous_point, current_point, color);
            previous_point = current_point;
        }

        // Small sphere marker at the segment end point.
        gizmos.sphere(Isometry3d::from_translation(previous_point), 0.5, color);
    }
}
