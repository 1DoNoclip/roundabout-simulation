use crate::*;

pub struct RoutePlugin;

impl Plugin for RoutePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Connection>()
            .register_type::<SpawnPoint>()
            .register_type::<EndPoint>();
    }
}

#[derive(Component, Reflect)]
#[reflect(Component, Default)]
/// A road segment
pub struct Segment {
    #[reflect(ignore)]
    /// The shape of the curve, where the f32 is the progress along the
    /// curve (between 0.0 and 1.0) and Vec3 is the result position.
    evaluator: Box<dyn Fn(f32) -> Vec3 + Send + Sync>,
    /// While length can be calculated automatically with curve.length()
    /// this is computationally expensive so it is only run once and cached.
    ///
    /// Performing curve.length() each frame for each segment is a
    /// huge waste of resources when the length does not change.
    length: f32,
    /// The maximum speed allowed in ms-1.
    speed_limit: f32,
}

impl Segment {
    pub fn new<C>(curve: C, speed_limit: f32) -> Self
    where
        C: enterpolation::Curve<f32, Output = Vec3> + Send + Sync + 'static,
    {
        let length = curve.length();
        Segment {
            evaluator: Box::new(move |time| curve.eval(time)),
            length,
            speed_limit,
        }
    }

    pub fn eval(&self, time: f32) -> Vec3 {
        (self.evaluator)(time)
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn speed_limit(&self) -> f32 {
        self.speed_limit
    }
}

// Default is required by reflect, should not be used manually.
impl Default for Segment {
    fn default() -> Self {
        Self {
            evaluator: Box::new(|_| Vec3::ZERO),
            ..default()
        }
    }
}

#[derive(Component, Reflect)]
/// Where road segments connect together, allowing vehicles to choose the next segment to use.
pub struct Connection {
    pub next_segments: Vec<Entity>,
    /// Determines whether the segment must yield to traffic on the new road.
    /// e.g., the entry into the roundabout requires yielding to circulating traffic.
    pub requires_yield: bool,
}

#[derive(Component, Reflect)]
/// Where vehicles spawn from.
pub struct SpawnPoint {
    pub vehicles_per_second: f32,
    /// The desirability of each destination from this spawn point.
    pub destination_weights: EntityHashMap<u32>,
}

#[derive(Component, Reflect)]
/// Where a vehicle may choose to head to.
pub struct EndPoint;

pub(super) fn draw_routes(mut gizmos: Gizmos, query: Query<&Segment>) {
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
