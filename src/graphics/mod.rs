use crate::*;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_routes, draw_vehicles).chain());
    }
}

pub fn draw_routes(mut gizmos: Gizmos, query: Query<&Segment>) {
    let resolution = 100;
    for segment in &query {
        let points = (0..=resolution)
            .map(|i| {
                let time = i as f32 / resolution as f32;
                (segment.evaluator)(time)
            })
            .collect::<Vec<_>>();
        gizmos.linestrip(points, Color::hsl(0.0, 0.0, 1.0));
    }
}

pub fn draw_vehicles(mut gizmos: Gizmos, vehicles: Query<&Transform, With<Navigator>>) {
    for transform in vehicles.iter() {
        // Draws a bright cyan circle with a 10.0 pixel/unit radius
        // at the vehicle's current coordinates.
        gizmos.circle_2d(
            transform.translation.truncate(),
            1.0,
            Color::linear_rgb(255.0, 0.0, 0.0),
        );
    }
}

pub fn draw_segments(mut gizmos: Gizmos, segments: Query<&Segment>) {
    const SAMPLE_STEPS: usize = 50;

    for segment in segments {
        let color = match segment.connection {
            Connection::NextSegments { requires_yield: true, .. } => Color::srgb(1.0, 0.2, 0.2),
            Connection::NextSegments { requires_yield: false, .. } => Color::srgb(0.2, 0.8, 1.0),
            Connection::EndPoint { .. } => Color::srgb(0.2, 1.0, 0.2),
        };

        let mut previous_point = (segment.evaluator)(0.0);
        for step in 1..=SAMPLE_STEPS {
            let time = step as f32 / SAMPLE_STEPS as f32;
            let current_point = (segment.evaluator)(time);

            gizmos.line(previous_point, current_point, color);
            previous_point = current_point;
        }

        let end_point = (segment.evaluator)(1.0);
        gizmos.sphere(Isometry3d::from_translation(end_point), 0.2, color);

        if let Connection::NextSegments { next_segments, .. } = &segment.connection {
            for next_entity in next_segments.iter() {
                if let Ok(next_segment) = segments.get(*next_entity) {
                    let next_start = (next_segment.evaluator)(0.0);
                    gizmos.line(end_point, next_start, Color::srgb(1.0, 0.9, 0.3));
                }
            }
        }
    }
}
