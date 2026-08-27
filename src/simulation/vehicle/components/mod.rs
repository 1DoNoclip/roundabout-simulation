use crate::*;

pub(super) struct ComponentsPlugin;

impl Plugin for ComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vehicle>()
            .register_type::<IdmDriver>()
            .register_type::<Kinematics>()
            .register_type::<Navigator>();
    }
}

/// Vehicle marker component;
#[derive(Component, Reflect)]
pub(crate) struct Vehicle;

/// The IDM values for the vehicle.
#[derive(Component, Reflect)]
pub(crate) struct IdmDriver {
    /// The desired speed of this vehicle out of the speed limit.
    desired_speed_percentage: f32,
    comfortable_acceleration: Acceleration,
    comfortable_deceleration: Acceleration,
    /// The minimum distance a vehicle will leave when stopping behind another stationary vehicle.
    minimum_gap: Distance,
    /// The time difference when following a vehicle.
    time_headway: Duration,
    /// The minimum time gap the driver will enter the roundabout in.
    critical_gap: Duration,
    exponent: f32,
}

impl IdmDriver {
    /// ### Arguments
    /// * `current_speed` - Speed of this vehicle.
    /// * `target_speed` - The speed that this vehicle achieves on an empty, straight road.
    /// * `lead_vehicle_info` - The information about the vehicle ahead (or a virtual yield vehicle).
    pub fn calculate_acceleration(
        &self,
        current_speed: Speed,
        target_speed: Speed,
        lead_vehicle_info: Option<LeadVehicleInfo>,
    ) -> Acceleration {
        let v = *current_speed;
        // Vehicles will drive at their desired_speed_percentage of the target speed.
        let v_0 = self.desired_speed_percentage * *target_speed;
        let a = *self.comfortable_acceleration;
        let b = *self.comfortable_deceleration;

        // Free road acceleration term.
        let free_road_term = 1.0 - (v / v_0).powf(self.exponent);

        let intersection_term = if let Some(lead_vehicle_info) = lead_vehicle_info {
            let s = *lead_vehicle_info.distance
                // If the lead vehicle is virtual (a yield line), then do not add a safety buffer.
                - match lead_vehicle_info.vehicle_kind {
                    VehicleKind::Real { length_metres } => length_metres + Kinematics::LENGTH_SAFETY_BUFFER_METRES,
                    VehicleKind::Virtual => 0.0,
                };
            let v_lead = *lead_vehicle_info.speed;
            let delta_v = v - v_lead;

            let dynamic_gap = (v * delta_v) / (2.0 * (a * b).sqrt());
            let s_star =
                *self.minimum_gap + (v * self.time_headway.as_secs_f32()) + (dynamic_gap.max(0.0));

            (s_star / s.max(0.1)).powi(2)
        } else {
            0.0
        };

        Acceleration::new_metres_per_second_squared(
            *self.comfortable_acceleration * (free_road_term - intersection_term),
        )
    }

    pub const fn critical_gap(&self) -> Duration {
        self.critical_gap
    }
}

impl Default for IdmDriver {
    fn default() -> Self {
        IdmDriver {
            desired_speed_percentage: 0.95,
            comfortable_acceleration: Acceleration::new_metres_per_second_squared(2.5),
            comfortable_deceleration: Acceleration::new_metres_per_second_squared(-2.0),
            minimum_gap: Distance::try_new_metres(2.0).expect("failed to create"),
            time_headway: Duration::from_secs_f32(1.5),
            critical_gap: Duration::from_secs_f32(3.5),
            exponent: 4.0,
        }
    }
}

/// Information about the vehicle in front of this vehicle.
pub(crate) struct LeadVehicleInfo {
    pub vehicle_kind: VehicleKind,
    /// Distance between front bumper of lead to front bumper of this vehicle.
    pub distance: Distance,
    /// The speed of the vehicle in front.
    pub speed: Speed,
}

pub(crate) enum VehicleKind {
    /// The positive length of the vehicle behind its front position (progress).
    Real {
        length_metres: f32,
    },
    Virtual,
}

/// The motion characteristics for the vehicle.
#[derive(Clone, Component, Copy, Reflect)]
pub(crate) struct Kinematics {
    /// Target speed that the driver would aim for on an empty road.
    target_speed: Speed,
    /// The maximum acceleration possible.
    max_acceleration: Acceleration,
    /// The maximum deceleration possible by braking.
    ///
    /// Use negative values as it is represented as an `Acceleration`.
    max_deceleration: Acceleration,
    /// The length of the vehicle behind its front position (progress).
    vehicle_length_metres: f32,
}

impl Kinematics {
    /// Behind vehicles will give an additional distance instead of drive into the lead vehicle's bumper.
    pub const LENGTH_SAFETY_BUFFER_METRES: f32 = 1.0;

    pub const fn new(
        target_speed: Speed,
        max_acceleration: Acceleration,
        max_deceleration: Acceleration,
    ) -> Self {
        Kinematics {
            target_speed,
            max_acceleration,
            max_deceleration,
            vehicle_length_metres: 4.4,
        }
    }

    pub fn calculate_time_to_arrival(
        current_speed: Speed,
        current_acceleration: Acceleration,
        distance: Distance,
    ) -> Option<Duration> {
        // Dereference newtypes into primitives.
        let (distance, speed, acceleration) = (*distance, *current_speed, *current_acceleration);

        // Uses s = ut + (1/2)at^2
        if acceleration.abs() > 0.01 {
            // If accelerating / decelerating significantly,
            // solve the quadratic equation. v^2 + 2as.
            let discriminant = speed * speed + 2.0 * acceleration * distance;
            if discriminant > 0.0 {
                let time = (-speed + discriminant.sqrt()) / acceleration;
                if time > 0.0 {
                    return Duration::try_from_secs_f32(time).ok();
                }
            }
        }

        // Fallback to using constant speed time-to-arrival.
        Duration::try_from_secs_f32(distance / speed).ok()
    }

    pub const fn target_speed(&self) -> Speed {
        self.target_speed
    }

    pub const fn max_acceleration(&self) -> Acceleration {
        self.max_acceleration
    }

    pub const fn max_deceleration(&self) -> Acceleration {
        self.max_deceleration
    }

    pub const fn vehicle_length_metres(&self) -> f32 {
        self.vehicle_length_metres
    }
}

/// Decides how the vehicle navigates the map.
#[derive(Component, Reflect)]
pub(crate) struct Navigator {
    /// The route for the vehicle to follow.
    route: Vec<Entity>,
    /// An index of route to identify the current segment.
    current_segment_index: usize,
    /// A segment progress between 0 and 1.
    progress: f32,
}

impl Navigator {
    pub fn try_new(route: Vec<Entity>) -> Result<Self, &'static str> {
        if route.is_empty() {
            Err("route cannot be empty")
        } else {
            Ok(Navigator {
                route,
                current_segment_index: 0,
                progress: 0.0,
            })
        }
    }

    /// Returns `Ok(())` if new index is in bounds of `self.route`.
    ///
    /// Returns `Err(())` if new index is out-of-bounds,
    /// meaning that the end of the route has been reached.
    pub(super) const fn increment_current_segment_index(&mut self) -> Result<(), ()> {
        if self.current_segment_index + 1 >= self.route.len() {
            Err(())
        } else {
            self.current_segment_index += 1;
            Ok(())
        }
    }

    /// Gets the current segment in the route.
    pub fn current_segment_id(&self) -> Entity {
        self.route[self.current_segment_index]
    }

    /// Returns `Ok(())` if `self.progress` < 1.0.
    ///
    /// Returns `Err(overflow_progress)` if `self.progress` >= 1.0 (the vehicle moves to the next segment).
    /// `overflow_progress` is the progress (in the current segment) that the vehicle is now in the next segment.
    pub const fn add_progress(&mut self, delta_progress: f32) -> Result<(), f32> {
        self.progress += delta_progress;
        if self.progress >= 1.0 {
            Err(self.progress - 1.0)
        } else {
            Ok(())
        }
    }

    /// Sets `self.progress` to 0.0.
    pub const fn reset_progress(&mut self) {
        self.progress = 0.0;
    }

    pub fn route(&self) -> &[Entity] {
        &self.route
    }

    pub const fn progress(&self) -> f32 {
        self.progress
    }
}
