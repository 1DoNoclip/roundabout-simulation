use bevy::prelude::*;
use enterpolation::{Signal, bspline::BSpline};

pub mod route;
pub mod vehicle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_simulation)
        .add_systems(Update, draw_routes)
        .run();
}

fn setup_simulation(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    let control_points = vec![
        Vec3::new(-300.0, -200.0, 0.0),
        Vec3::new(0.0, 200.0, 0.0),
        Vec3::new(300.0, -200.0, 0.0),
    ];

    let bspline = BSpline::builder()
        .clamped()
        .elements(control_points)
        .equidistant::<f32>()
        .degree(2)
        .domain(-2.0, 2.0)
        .constant::<4>()
        .build()
        .unwrap();

    commands.spawn(RoundaboutRoute {
        evaluator: Box::new(move |time| bspline.eval(time)),
    });
}

fn draw_routes(mut gizmos: Gizmos, query: Query<&RoundaboutRoute>) {
    let resolution = 100;
    for route in &query {
        let points = (0..=resolution)
            .map(|i| {
                let time = i as f32 / resolution as f32;
                (route.evaluator)(time)
            })
            .collect::<Vec<_>>();
        gizmos.linestrip(points, Color::hsl(120.0, 1.0, 0.5));
    }
}

#[derive(Resource)]
struct RoundaboutRoute {
    evaluator: Box<dyn Fn(f32) -> Vec3 + Send + Sync>,
}
