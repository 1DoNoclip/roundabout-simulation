use crate::*;

pub struct BlueprintPlugin;

impl Plugin for BlueprintPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<IntersectionBlueprint>()
            .register_type::<RoundaboutCircleBlueprint>()
            .register_type::<ArmBlueprint>();
    }
}

/// Represents global intersection data.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct IntersectionBlueprint {
    /// Length between 3 -> 6.
    /// Max 2 lanes allowed when there are 3 arms.
    pub arms: Vec<ArmBlueprint>,
    /// A number 1 -> 3.
    /// The number of lanes for each carriageway (entry and exit roads, including roundabout circle).
    pub number_of_lanes: usize,
    /// Maximum speed for vehicles to travel at.
    /// Default speed limit of the roundabout and arms in ms-1.
    /// Can be overridden on individual arms.
    pub speed_limit: Speed,
    /// A greater deflection radius causes a smoother entry onto the roundabout.
    /// Increases capacity and reduces safety by increasing entry speeds.
    pub deflection_radius: f32,
}

impl IntersectionBlueprint {
    pub fn try_new(
        arms: Vec<ArmBlueprint>,
        number_of_lanes: usize,
        speed_limit: Speed,
        deflection_radius: f32,
    ) -> Result<Self, String> {
        let arms_length = arms.len();
        if !(3..=6).contains(&arms_length) {
            return Err(format!(
                "length of arms must be between 3 and 6 inclusive, found {arms_length}"
            ));
        }
        if !(1..=3).contains(&number_of_lanes) {
            return Err(format!(
                "number_of_lanes must be between 1 and 3 inclusive, found {number_of_lanes}"
            ));
        }
        if deflection_radius <= 0.0 || deflection_radius.is_nan() {
            return Err(format!(
                "deflection_radius must be positive, found {deflection_radius}"
            ));
        }
        Ok(IntersectionBlueprint {
            arms,
            number_of_lanes,
            speed_limit,
            deflection_radius,
        })
    }
}

/// Represents the circular part of the roundabout.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct RoundaboutCircleBlueprint {
    /// Radius of the inner roundabout circle in metres.
    /// The distance between the centre and the centre of the inner circulating lane.
    pub radius: f32,
}

impl RoundaboutCircleBlueprint {
    pub fn try_new(radius: f32) -> Result<Self, String> {
        if radius <= 0.0 || radius.is_nan() {
            return Err(format!("radius must be positive, found {radius}"));
        }
        Ok(RoundaboutCircleBlueprint { radius })
    }
}

/// Represent a singular arm to the roundabout.
#[derive(Clone, Component, Copy, Reflect)]
pub struct ArmBlueprint {
    /// The angle of the arm to the roundabout.
    pub angle: Rot2,
    /// Maximum speed for vehicles to travel at.
    /// Overrides the global `IntersectionBlueprint` resource's speed limit for this arm.
    pub speed_limit_override: Option<Speed>,
    /// The maximum vehicles spawned per second. The actual spawn rate may
    /// be less due to lack of space in the network to spawn another vehicle.
    pub max_vehicles_per_second: f32,
}

impl ArmBlueprint {
    pub fn from_degrees(
        degrees: f32,
        speed_limit_override: Option<Speed>,
        max_vehicles_per_second: f32,
    ) -> Self {
        ArmBlueprint {
            angle: Rot2::degrees(degrees),
            speed_limit_override,
            max_vehicles_per_second,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_arm_blueprint() {
        ArmBlueprint::from_degrees(90.0, None, 0.5);
    }

    #[test]
    fn try_new_roundabout_circle_blueprint() {
        RoundaboutCircleBlueprint::try_new(30.0).expect("failed to create");
    }

    #[test]
    fn try_new_intersection_blueprint() {
        let arms = vec![
            ArmBlueprint::from_degrees(0.0, None, 0.5),
            ArmBlueprint::from_degrees(90.0, None, 0.5),
            ArmBlueprint::from_degrees(180.0, None, 0.5),
        ];
        let number_of_lanes = 2;
        let speed_limit = Speed::from_miles_per_hour(30.0).expect("failed to create");
        let deflection_radius = 15.0;

        IntersectionBlueprint::try_new(arms, number_of_lanes, speed_limit, deflection_radius)
            .expect("failed to create");
    }
}
