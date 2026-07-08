use crate::*;

#[derive(Resource, Reflect)]
/// Represents global intersection data.
pub struct IntersectionBlueprint {
    /// A number 1 -> 3.
    /// If number_of_entry_lanes == 3, then there is a dedicated left turn lane and
    /// exit lane separate to the roundabout.
    /// This dedicated lane does not need to yield to roundabout traffic,
    /// although will have to yield when merging back on the exit road.
    pub number_of_entry_lanes: usize,
    /// A number 3 -> 6.
    /// Max 2 entry lanes allowed when there are 3 arms.
    pub number_of_arms: usize,
    /// Speed limit in ms-1
    pub speed_limit: f32,
    /// A greater deflection radius causes a smoother entry onto the roundabout.
    /// Increases capacity and reduces safety by increasing entry speeds.
    pub deflection_radius: f32,
}

#[derive(Resource, Reflect)]
/// Represents the circular part of the roundabout.
pub struct RoundaboutCircleBlueprint {
    /// Radius of the inner roundabout circle in metres.
    pub radius: f32,
}

#[derive(Component, Reflect)]
/// Represent a singular arm to the roundabout.
pub struct ArmBlueprint {
    /// In degrees / °.
    angle: f32,
}
