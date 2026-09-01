//! Contains the components used in the roundabout layout, such as segments, connections and end points.

use crate::*;

pub(crate) mod segment_type;

pub(super) struct ComponentsPlugin;

impl Plugin for ComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Arm>()
            .register_type::<Connection>()
            .register_type::<EndPoint>()
            .register_type::<Segment>()
            .register_type::<SpawnPoint>();
    }
}

#[derive(Bundle)]
pub(crate) struct ArmBundle {
    name: Name,
    arm: Arm,
    vehicle_spawn_queue: VehicleSpawnQueue,
}

impl ArmBundle {
    pub fn new(
        index: usize,
        angle: Rot2,
        max_vehicles_per_second: f32,
        destination_weights: DestinationWeights,
    ) -> Self {
        ArmBundle {
            name: Name::new(format!("Arm: [{index}]")),
            arm: Arm::new(index, angle, max_vehicles_per_second, destination_weights),
            vehicle_spawn_queue: VehicleSpawnQueue::new(),
        }
    }
}

pub(crate) type DestinationWeights = EntityHashMap<u32>;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
/// A marker for spawned road `Segment`s and other components to assign themselves to.
///
/// E.g., a `SpawnPoint` and `EndPoint` point to an `Arm` entity.
pub(crate) struct Arm {
    /// The unique index of this arm.
    index: usize,
    /// The angle of the arm to the roundabout.
    angle: Rot2,
    /// The maximum vehicles spawned per second. The actual spawn rate may
    /// be less due to lack of space in the network to spawn another vehicle.
    max_vehicles_per_second: f32,
    /// The desirability of each destination from this spawn point.
    destination_weights: DestinationWeights,
}

impl Arm {
    pub const fn new(
        index: usize,
        angle: Rot2,
        max_vehicles_per_second: f32,
        destination_weights: DestinationWeights,
    ) -> Self {
        Arm {
            index,
            angle,
            max_vehicles_per_second,
            destination_weights,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn max_vehicles_per_second(&self) -> f32 {
        self.max_vehicles_per_second
    }

    pub const fn destination_weights(&self) -> &DestinationWeights {
        &self.destination_weights
    }
}

/// A road segment between connections.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct Segment {
    /// The position, tangent and curvature evaluator functions.
    #[reflect(ignore)]
    evaluators: Evaluators,

    /// While the start position can be calculated automatically with `self.sample_clamped(0.0)`,
    /// this is ran multiple times per second when spawning vehicles and always gives the same result.
    start_position: Vec3,

    /// The `Arm` that this `Segment` is on.
    arm_id: Entity,

    /// The arm index that this `Segment` is on.
    arm_index: usize,

    /// The lane index that this `Segment` is on.
    lane_index: usize,

    /// The next segments / end point that this segment connects to.
    connection: Connection,

    /// While length can be calculated automatically with `self.length()`,
    /// this is computationally expensive so it is only run once and cached.
    ///
    /// Performing `self.length_metres()` each frame for each segment is a
    /// huge waste of resources when the length does not change after creation.
    length_metres: f32,

    /// The maximum speed allowed for vehicles to travel at.
    speed_limit: Speed,
}

impl Segment {
    pub fn new<C: SegmentCurve>(
        curve: C,
        arm_id: Entity,
        arm_index: usize,
        lane_index: usize,
        connection: Connection,
        speed_limit: Speed,
    ) -> Self {
        let length_metres = curve.length_metres();
        let evaluators = curve.into_evaluators();
        let start_position = evaluators.progress_at(0.0);
        Segment {
            evaluators,
            start_position,
            arm_id,
            arm_index,
            lane_index,
            connection,
            length_metres,
            speed_limit,
        }
    }

    pub fn progress_at(&self, time: f32) -> Vec3 {
        self.evaluators.progress_at(time)
    }

    pub const fn start_position(&self) -> Vec3 {
        self.start_position
    }

    pub const fn arm_id(&self) -> Entity {
        self.arm_id
    }

    pub const fn arm_index(&self) -> usize {
        self.arm_index
    }

    pub const fn lane_index(&self) -> usize {
        self.lane_index
    }

    pub const fn connection(&self) -> &Connection {
        &self.connection
    }

    pub const fn length_metres(&self) -> f32 {
        self.length_metres
    }
}

// Default is required by Reflect, should not be used manually.
impl Default for Segment {
    fn default() -> Self {
        Self {
            // The evaluator's type does not implement Default, so cannot derive Default.
            evaluators: Evaluators::dummy(),
            ..default()
        }
    }
}

/// Where road segments connect together, allowing vehicles to choose the next segment to use, or exit the map.
#[derive(Debug, Reflect)]
pub(crate) enum Connection {
    /// A direct connection from one segment to another.
    Direct { next_segment_id: Entity },
    /// An exit from the roundabout while a lane still circulates.
    Diverge {
        exit_arm_index: usize,
        exit_segment_id: Entity,
        circulating_segment_id: Entity,
    },
    /// This connection exits the map.
    EndPoint { end_point_id: Entity },
    /// A merge onto the roundabout, requiring a yield at the designated yield point.
    Merge { next_segment_id: Entity },
}

/// Where vehicles spawn from.
#[derive(Component, Debug, Reflect)]
pub(crate) struct SpawnPoint {
    /// The arm that this `SpawnPoint`'s road lies on.
    arm: Entity,
    /// The index of the lane this `SpawnPoint` connects to.
    lane_index: usize,
    /// The `Segment` that this `SpawnPoint` attaches to.
    /// Vehicles will spawn from this `SpawnPoint` and immediately
    /// begin moving along this `Segment`.
    segment: Entity,
}

impl SpawnPoint {
    pub const fn new(arm: Entity, lane_index: usize, segment: Entity) -> Self {
        SpawnPoint {
            arm,
            lane_index,
            segment,
        }
    }

    pub const fn arm(&self) -> Entity {
        self.arm
    }

    pub const fn lane_index(&self) -> usize {
        self.lane_index
    }

    pub const fn segment(&self) -> Entity {
        self.segment
    }
}

/// Where a vehicle may choose to head to.
#[derive(Component, Debug, Reflect)]
pub(crate) struct EndPoint {
    /// The arm that this `EndPoint`'s road lies on.
    arm: Entity,
    /// The index of the lane this `EndPoint` is connected to.
    lane_index: usize,
}

impl EndPoint {
    pub const fn new(arm: Entity, lane_index: usize) -> Self {
        EndPoint { arm, lane_index }
    }

    pub const fn arm(&self) -> Entity {
        self.arm
    }
}
