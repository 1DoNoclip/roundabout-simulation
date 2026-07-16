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
