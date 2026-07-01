use bevy::{ecs::entity::EntityHashMap, prelude::*};

#[derive(Component, Reflect)]
#[reflect(Component, Default)]
/// A road segment
pub struct Segment {
    #[reflect(ignore)]
    /// The shape of the curve, where the f32 is the progress along the
    /// curve (between 0.0 and 1.0) and Vec3 is the result position.
    pub evaluator: Box<dyn Fn(f32) -> Vec3 + Send + Sync>,
    pub length: f32,
    pub speed_limit: f32,
}

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
