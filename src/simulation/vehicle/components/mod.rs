use crate::*;

pub(super) struct ComponentsPlugin;

impl Plugin for ComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<IdmDriver>()
            .register_type::<Kinematics>()
            .register_type::<Navigator>();
    }
}

/// The IDM values for the vehicle.
#[derive(Component, Reflect)]
pub(crate) struct IdmDriver {
    desired_speed: Speed,
    comfortable_acceleration: Acceleration,
    comfortable_deceleration: Acceleration,
    minimum_gap: Distance,
    time_headway: Duration,
    exponent: f32,
}

impl IdmDriver {
    /// ### Arguments
    /// * `current_speed` - Speed of this vehicle.
    /// * `lead_vehicle` - (distance to lead, speed of lead).
    pub fn calculate_acceleration(
        &self,
        current_speed: Speed,
        lead_vehicle_info: Option<LeadVehicleInfo>,
    ) -> Acceleration {
        let v = *current_speed;
        let v_0 = *self.desired_speed;
        // Free road acceleration term.
        let free_road_term = 1.0 - (v / v_0).powf(self.exponent);
        let intersection_term = if let Some(lead_vehicle_info) = lead_vehicle_info {
            let s = lead_vehicle_info.distance;
            let v_lead = lead_vehicle_info.speed;
            let delta_v = v - *v_lead;
            let s_star = *self.minimum_gap
                + (v * self.time_headway.as_secs_f32())
                + (v * delta_v)
                    / (2.0 * *self.comfortable_acceleration * *self.comfortable_deceleration)
                        .sqrt();

            s_star / s.max(0.1).powi(2)
        } else {
            0.0
        };

        Acceleration::new(*self.comfortable_acceleration * (free_road_term - intersection_term))
    }
}

impl Default for IdmDriver {
    fn default() -> Self {
        IdmDriver {
            desired_speed: Speed::from_miles_per_hour(30.0).expect("failed to create"),
            comfortable_acceleration: Acceleration::new(2.5),
            comfortable_deceleration: Acceleration::new(-2.0),
            minimum_gap: Distance::new(2.0).expect("failed to create"),
            time_headway: Duration::from_secs_f32(1.5),
            exponent: 4.0,
        }
    }
}

/// Information about the vehicle in front of this vehicle.
pub(crate) struct LeadVehicleInfo {
    /// Distance between rear bumper of lead to front bumper of this vehicle.
    pub distance: Distance,
    /// The speed of the vehicle in front.
    pub speed: Speed,
}

/// The motion characteristics for the vehicle.
#[derive(Component, Reflect)]
pub(crate) struct Kinematics {
    /// The current speed of the vehicle.
    pub speed: Speed,
    /// Target speed that the driver would aim for on an empty road.
    target_speed: Speed,
    max_acceleration: Acceleration,
    /// The maximum deceleration possible by braking.
    max_deceleration: Acceleration,
}

impl Kinematics {
    pub const fn new(
        speed: Speed,
        target_speed: Speed,
        max_acceleration: Acceleration,
        max_deceleration: Acceleration,
    ) -> Self {
        Kinematics {
            speed,
            target_speed,
            max_acceleration,
            max_deceleration,
        }
    }

    pub const fn target_speed(&self) -> Speed {
        self.target_speed
    }

    pub const fn max_acceleration(&self) -> Acceleration {
        self.max_acceleration
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
        self.current_segment_index += 1;
        if self.current_segment_index >= self.route.len() {
            Err(())
        } else {
            Ok(())
        }
    }

    /// Gets the current segment in the route.
    ///
    /// Returns `Some<Entity>` if the current segment is valid.
    ///
    /// Returns `None` if the current segment index is out of bounds (due to reaching end of route).
    pub fn current_segment_id(&self) -> Option<Entity> {
        self.route.get(self.current_segment_index).copied()
    }

    /// Returns `Ok(())` if `self.progress` < 1.0.
    ///
    /// Returns `Err(())` if `self.progress` >= 1.0 (the vehicle moves to the next segment).
    pub const fn add_progress(&mut self, delta_progress: f32) -> Result<(), ()> {
        self.progress += delta_progress;
        if self.progress >= 1.0 {
            Err(())
        } else {
            Ok(())
        }
    }

    /// Sets `self.progress` to 0.0.
    pub const fn reset_progress(&mut self) {
        self.progress = 0.0;
    }

    pub const fn progress(&self) -> f32 {
        self.progress
    }
}
