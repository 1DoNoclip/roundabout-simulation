use crate::*;

pub mod speed;

pub use speed::*;

pub struct ComponentsPlugin;

impl Plugin for ComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SpeedPlugin)
            .register_type::<Arm>()
            .register_type::<Connection>()
            .register_type::<EndPoint>()
            .register_type::<Segment>()
            .register_type::<SpawnPoint>();
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Arm {
    /// The unique index of this arm.
    pub index: usize,
}

impl Arm {
    pub fn new(index: usize) -> Self {
        Arm { index }
    }
}

/// A road segment between connections.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub struct Segment {
    /// The shape of the curve, where the f32 is the progress along the
    /// curve (between 0.0 and 1.0) and Vec3 is the result position.
    #[reflect(ignore)]
    pub evaluator: Box<dyn Fn(f32) -> Vec3 + Send + Sync>,

    /// The next segments / end point that this segment connects to.
    pub connection: Connection,

    /// While length can be calculated automatically with curve.length()
    /// this is computationally expensive so it is only run once and cached.
    ///
    /// Performing curve.length() each frame for each segment is a
    /// huge waste of resources when the length does not change after creation.
    pub length: f32,

    /// The maximum speed allowed for vehicles to travel at.
    pub speed_limit: Speed,
}

impl Segment {
    pub fn new<C: SegmentCurve>(curve: C, connection: Connection, speed_limit: Speed) -> Self {
        let length = curve.length();
        Segment {
            evaluator: curve.into_evaluator(),
            connection,
            length,
            speed_limit,
        }
    }

    pub fn to_end<C: SegmentCurve>(curve: C, end_point: Entity, speed_limit: Speed) -> Self {
        let connection = Connection::EndPoint { end_point };
        Segment::new(curve, connection, speed_limit)
    }

    pub fn sample_clamped(&self, time: f32) -> Vec3 {
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

/// Where road segments connect together, allowing vehicles to choose the next segment to use, or exit the map.
#[derive(Reflect)]
pub enum Connection {
    /// This connection connects to other segments.
    NextSegments {
        next_segments: Vec<Entity>,
        /// Determines whether the segment must yield to traffic on the new road.
        /// e.g., the entry into the roundabout requires yielding to circulating traffic.
        requires_yield: bool,
    },
    /// This connection exits the map.
    EndPoint { end_point: Entity },
}

/// Where vehicles spawn from.
#[derive(Component, Reflect)]
pub struct SpawnPoint {
    /// The arm that this `SpawnPoint`'s road lies on.
    pub arm: Entity,
    /// The segment that this spawn point is attached to.
    pub segment: Entity,
    /// The maximum vehicles spawned per second. The actual spawn rate may
    /// be less due to lack of space in the network to spawn another vehicle.
    pub max_vehicles_per_second: f32,
    /// The desirability of each destination from this spawn point.
    pub destination_weights: EntityHashMap<u32>,
}

/// Where a vehicle may choose to head to.
#[derive(Component, Reflect)]
pub struct EndPoint {
    /// The arm that this `EndPoint`'s road lies on.
    pub arm: Entity,
}
