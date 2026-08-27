use crate::*;
use bevy::math::FloatOrd;

pub(crate) struct BlueprintPlugin;

impl Plugin for BlueprintPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ArmBlueprint>()
            .register_type::<CircleBlueprint>()
            .register_type::<RoundaboutBlueprint>();
    }
}

/// Represents global roundabout data.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(crate) struct RoundaboutBlueprint {
    /// Length between 3 -> 6.
    /// Max 2 lanes allowed when there are 3 arms.
    ///
    /// Must be sorted in reverse angle order.
    arm_blueprints: Vec<ArmBlueprint>,
    /// The blueprint for the circle itself.
    circle_blueprint: CircleBlueprint,
    /// A number 1 -> 3.
    /// The number of lanes for each carriageway (entry and exit roads, including roundabout circle).
    number_of_lanes: usize,
    /// Maximum speed for vehicles to travel at.
    /// Default speed limit of the roundabout and arms in ms-1.
    /// Can be overridden on individual arms.
    speed_limit: Speed,
}

impl RoundaboutBlueprint {
    /// Attempts to create a blueprint.
    ///
    /// Sorts arms into reverse angle order for correct creation.
    pub fn try_new(
        mut arm_blueprints: Vec<ArmBlueprint>,
        circle_blueprint: CircleBlueprint,
        number_of_lanes: usize,
        speed_limit: Speed,
    ) -> Result<Self, String> {
        let arms_length = arm_blueprints.len();
        if !(3..=6).contains(&arms_length) {
            Err(format!(
                "length of arm_blueprints must be between 3 and 6 inclusive, found {arms_length}"
            ))
        } else if !(1..=3).contains(&number_of_lanes) {
            Err(format!(
                "number_of_lanes must be between 1 and 3 inclusive, found {number_of_lanes}"
            ))
        } else {
            // Assembly expects the arm blueprints to be sorted in reverse angular order.
            arm_blueprints
                .sort_by_cached_key(|arm| std::cmp::Reverse(FloatOrd(arm.angle.as_radians())));

            Ok(RoundaboutBlueprint {
                arm_blueprints,
                circle_blueprint,
                number_of_lanes,
                speed_limit,
            })
        }
    }

    pub const fn arm_blueprints(&self) -> &[ArmBlueprint] {
        // Using &self.arm_blueprints is not yet stable const.
        // .as_slice is directly implemented on a Vec, but the &[]
        // slice is a trait-based implementation.
        self.arm_blueprints.as_slice()
    }

    pub const fn number_of_arms(&self) -> usize {
        self.arm_blueprints.len()
    }

    pub const fn circle_blueprint(&self) -> &CircleBlueprint {
        &self.circle_blueprint
    }

    pub const fn number_of_lanes(&self) -> usize {
        self.number_of_lanes
    }

    pub const fn speed_limit(&self) -> Speed {
        self.speed_limit
    }
}

/// Represents a singular arm on the roundabout.
#[derive(Reflect)]
pub(crate) struct ArmBlueprint {
    /// The angle of the arm to the roundabout.
    angle: Rot2,
    /// Maximum speed for vehicles to travel at.
    /// Overrides the global `IntersectionBlueprint` resource's speed limit for this arm.
    speed_limit_override: Option<Speed>,
    /// The maximum vehicles spawned per second. The actual spawn rate may
    /// be less due to lack of space in the network to spawn another vehicle.
    max_vehicles_per_second: f32,
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

    pub const fn angle(&self) -> Rot2 {
        self.angle
    }

    pub const fn speed_limit_override(&self) -> Option<Speed> {
        self.speed_limit_override
    }

    pub const fn max_vehicles_per_second(&self) -> f32 {
        self.max_vehicles_per_second
    }
}

/// Represents the circular part of the roundabout.
#[derive(Reflect)]
pub(crate) struct CircleBlueprint {
    /// Radius of the inner roundabout circle in metres.
    /// The distance between the centre and the centre of the inner circulating lane.
    radius: f32,
    /// A greater deflection radius causes a smoother entry onto the roundabout.
    /// Increases capacity and reduces safety by increasing entry speeds.
    deflection_radius: f32,
}

impl CircleBlueprint {
    pub fn try_new(radius: f32, deflection_radius: f32) -> Result<Self, String> {
        if radius <= 0.0 || radius.is_nan() {
            Err(format!("radius must be positive, found {radius}"))
        } else if deflection_radius <= 0.0 || deflection_radius.is_nan() {
            Err(format!(
                "deflection_radius must be positive, found {deflection_radius}"
            ))
        } else if deflection_radius > radius {
            Err(format!(
                "deflection_radius ({deflection_radius}) cannot exceed radius ({radius})"
            ))
        } else {
            Ok(CircleBlueprint {
                radius,
                deflection_radius,
            })
        }
    }

    pub const fn radius(&self) -> f32 {
        self.radius
    }

    pub const fn deflection_radius(&self) -> f32 {
        self.deflection_radius
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
        CircleBlueprint::try_new(30.0, 15.0).expect("failed to create");
    }

    #[test]
    fn try_new_roundabout_blueprint() {
        let arms = vec![
            ArmBlueprint::from_degrees(0.0, None, 0.5),
            ArmBlueprint::from_degrees(90.0, None, 0.5),
            ArmBlueprint::from_degrees(180.0, None, 0.5),
        ];
        let circle_blueprint = CircleBlueprint::try_new(30.0, 15.0).expect("failed to create");
        let number_of_lanes = 2;
        let speed_limit = Speed::try_miles_per_hour(30.0).expect("failed to create");

        RoundaboutBlueprint::try_new(arms, circle_blueprint, number_of_lanes, speed_limit)
            .expect("failed to create");
    }
}
