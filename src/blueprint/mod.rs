use crate::*;

pub struct BlueprintPlugin;

impl Plugin for BlueprintPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<IntersectionBlueprint>()
            .register_type::<RoundaboutCircleBlueprint>()
            .register_type::<ArmBlueprint>();
    }
}

#[derive(Resource, Reflect)]
/// Represents global intersection data.
pub struct IntersectionBlueprint {
    /// Length between 3 -> 6.
    /// Max 2 entry lanes allowed when there are 3 arms.
    pub arms: Vec<ArmBlueprint>,
    /// A number 1 -> 3.
    /// The number of lanes for each carriageway (entry and exit roads, including roundabout circle).
    pub number_of_lanes: usize,
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
