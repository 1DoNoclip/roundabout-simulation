use bevy::{ecs::entity::EntityHashMap, prelude::*};

#[derive(Component)]
/// A road segment
pub struct Segment {
    /// The shape of the curve, where the f32 is the progress along the curve (between 0.0 and 1.0) and Vec3 is the result position
    pub evaluator: Box<dyn Fn(f32) -> Vec3 + Send + Sync>,
    pub length: f32,
    pub speed_limit: f32,
}

#[derive(Component)]
/// Where road segments connect together, allowing vehicles to choose the next segment to use
pub struct Connection {
    pub next_segments: Vec<Entity>,
    /// e.g., the entry into the roundabout requires yielding to circulating traffic
    pub requires_yield: bool,
}

#[derive(Component)]
/// Where vehicles spawn from
pub struct SpawnPoint {
    pub vehicles_per_second: f32,
    /// The desirability of each destination from this spawn point
    pub destination_weights: EntityHashMap<u32>,
}

#[derive(Component)]
/// Where a vehicle may choose to head to
pub struct EndPoint;
