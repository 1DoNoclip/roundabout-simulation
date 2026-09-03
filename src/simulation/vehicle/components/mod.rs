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
    #[reflect(ignore)]
    comfortable_acceleration: Acceleration,
    #[reflect(ignore)]
    comfortable_deceleration: Acceleration,
    /// The minimum distance a vehicle will leave when stopping behind another stationary vehicle.
    #[reflect(ignore)]
    minimum_gap: Distance,
    /// The time difference when following a vehicle.
    #[reflect(ignore)]
    time_headway: UomTime,
    /// The minimum time gap the driver will enter the roundabout in.
    #[reflect(ignore)]
    critical_gap: UomTime,
    exponent: f32,
}

impl IdmDriver {
    /// ### Arguments
    /// * `current_speed` - Speed of this vehicle.
    /// * `target_speed` - The speed that this vehicle achieves on an empty, straight road.
    /// * `lead_vehicle_info` - The information about the vehicle ahead (or a virtual yield line vehicle).
    pub fn calculate_acceleration(
        &self,
        current_speed: Velocity,
        target_speed: Velocity,
        lead_vehicle_info: Option<LeadVehicleInfo>,
    ) -> Acceleration {
        let v = current_speed.get::<meter_per_second>();
        // Vehicles will drive at their desired_speed_percentage of the target speed.
        let v_0 = self.desired_speed_percentage * target_speed.get::<meter_per_second>();
        let a = self
            .comfortable_acceleration
            .get::<meter_per_second_squared>();
        let b = self
            .comfortable_deceleration
            .get::<meter_per_second_squared>();

        // Free road acceleration term.
        let free_road_term = 1.0 - (v / v_0).powf(self.exponent);

        let intersection_term = if let Some(lead_vehicle_info) = lead_vehicle_info {
            // The actual distance to the lead vehicle's rear bumper.
            // let s = *lead_vehicle_info.distance
            //     // If the lead vehicle is virtual (a yield line), then do not add a safety buffer.
            //     - match lead_vehicle_info.vehicle_kind {
            //         VehicleKind::Real { length_metres } => length_metres,
            //         VehicleKind::Virtual => 0.0,
            //     };
            let (s, minimum_gap) = match lead_vehicle_info.vehicle_kind {
                VehicleKind::Real(length) => (
                    (*lead_vehicle_info.distance - length).get::<meter>(),
                    self.minimum_gap,
                ),
                VehicleKind::Virtual => (lead_vehicle_info.distance.get::<meter>(), Distance::ZERO),
            };
            let v_lead = lead_vehicle_info.speed.get::<meter_per_second>();
            let delta_v = v - v_lead;

            let dynamic_gap = (v * delta_v) / (2.0 * (a * b).sqrt());
            // The desired distance for this vehicle to have to the lead vehicle.
            let s_star = (v * self.time_headway.get::<second>())
                + (dynamic_gap.max(0.0))
                // If the lead vehicle is actually a yield line, then allow
                // this vehicle to pull right up to it, ignoring `minimum_gap`.
                + minimum_gap.get::<meter>();

            (s_star / s.max(0.1)).powi(2)
        } else {
            0.0
        };

        self.comfortable_acceleration * (free_road_term - intersection_term)
    }

    pub const fn critical_gap(&self) -> UomTime {
        self.critical_gap
    }
}

impl Default for IdmDriver {
    fn default() -> Self {
        IdmDriver {
            desired_speed_percentage: 0.95,
            comfortable_acceleration: Acceleration::new::<meter_per_second_squared>(2.5),
            comfortable_deceleration: Acceleration::new::<meter_per_second_squared>(-2.0),
            minimum_gap: Distance::try_new(Length::new::<meter>(2.0)).expect("failed to create"),
            time_headway: UomTime::new::<second>(1.5),
            critical_gap: UomTime::new::<second>(3.5),
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
    Real(Length),
    Virtual,
}

/// The motion characteristics for the vehicle.
#[derive(Clone, Component, Copy, Reflect)]
pub(crate) struct Kinematics {
    /// Target speed that the driver would aim for on an empty road.
    target_speed: Speed,
    /// The maximum acceleration possible.
    #[reflect(ignore)]
    max_acceleration: Acceleration,
    /// The maximum deceleration possible by braking.
    ///
    /// Use negative values as it is represented as an `Acceleration`.
    #[reflect(ignore)]
    max_deceleration: Acceleration,
    /// The length of the vehicle behind its front position (progress).
    #[reflect(ignore)]
    vehicle_length: Length,
}

impl Kinematics {
    pub fn new(
        target_speed: Speed,
        max_acceleration: Acceleration,
        max_deceleration: Acceleration,
    ) -> Self {
        Kinematics {
            target_speed,
            max_acceleration,
            max_deceleration,
            vehicle_length: Length::new::<meter>(4.4),
        }
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

    pub const fn vehicle_length(&self) -> Length {
        self.vehicle_length
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
    /// ### Panics
    /// May panic if `self.current_segment_index` is out of bounds, such as reaching the end of the route.
    pub fn current_segment_id(&self) -> Entity {
        self.route[self.current_segment_index]
    }

    /// Tries to get the next segment in the route.
    /// ### Returns
    /// * `Some(Entity)` - If the next segment is valid.
    /// * `None` - If the current segment is the last segment.
    pub fn next_segment_id(&self) -> Option<Entity> {
        self.route.get(self.current_segment_index + 1).copied()
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
