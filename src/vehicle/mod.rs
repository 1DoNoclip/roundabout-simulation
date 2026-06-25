use bevy::prelude::*;

#[derive(Component, Reflect)]
/// The motion characteristics for the vehicle
pub struct Kinematics {
    pub speed: f32,
    /// Target speed that the driver would aim for on an empty road
    pub target_speed: f32,
    pub acceleration: f32,
}

#[derive(Component, Reflect)]
/// Decides how the vehicle navigates the map
pub struct Navigator {
    /// The route for the vehicle to follow
    pub route: Vec<Entity>,
    /// An index of route to identify the current segment
    pub current_segment: usize,
    /// A segment progress between 0 and 1
    pub progress: f32,
}
