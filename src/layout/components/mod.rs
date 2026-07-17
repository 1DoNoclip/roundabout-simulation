use crate::*;

pub mod speed_limit;

pub use speed_limit::*;

pub struct ComponentsPlugin;

impl Plugin for ComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SpeedLimitPlugin)
            .register_type::<Connection>()
            .register_type::<SpawnPoint>()
            .register_type::<EndPoint>();
    }
}

/// The width of a singular lane of roads and roundabout in metres.
pub const LANE_WIDTH: f32 = 3.5;

#[derive(Component, Reflect)]
#[reflect(Component, Default)]
/// A road segment between connections.
pub struct Segment {
    #[reflect(ignore)]
    /// The shape of the curve, where the f32 is the progress along the
    /// curve (between 0.0 and 1.0) and Vec3 is the result position.
    pub evaluator: Box<dyn Fn(f32) -> Vec3 + Send + Sync>,
    /// While length can be calculated automatically with curve.length()
    /// this is computationally expensive so it is only run once and cached.
    ///
    /// Performing curve.length() each frame for each segment is a
    /// huge waste of resources when the length does not change.
    pub length: f32,
    /// The maximum speed allowed in ms-1.
    pub speed_limit: SpeedLimit,
}

impl Segment {
    pub fn new<C>(curve: C, speed_limit: SpeedLimit) -> Self
    where
        C: CurveLength + Send + Sync + 'static,
    {
        let length = curve.length();
        Segment {
            evaluator: Box::new(move |time| curve.sample_clamped(time)),
            length,
            speed_limit,
        }
    }

    pub fn eval(&self, time: f32) -> Vec3 {
        (self.evaluator)(time)
    }
}

// Default is required by reflect, should not be used manually.
impl Default for Segment {
    fn default() -> Self {
        Self {
            // The evaluator's type does not implement Default, so cannot derive Default.
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
    /// The maximum vehicles spawned per second. The actual spawn rate may
    /// be less due to lack of space in the network to spawn another vehicle.
    pub max_vehicles_per_second: f32,
    /// The desirability of each destination from this spawn point.
    pub destination_weights: EntityHashMap<u32>,
}

#[derive(Component, Reflect)]
/// Where a vehicle may choose to head to.
pub struct EndPoint;
