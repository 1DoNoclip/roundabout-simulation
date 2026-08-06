use crate::*;

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

    pub fn current_segment_id(&self) -> Entity {
        *self
            .route
            .get(self.current_segment_index)
            .expect("current_segment_index should be in range of route")
    }

    pub const fn add_progress(&mut self, delta_progress: f32) -> Result<(), ()> {
        self.progress += delta_progress;
        if self.progress >= 1.0 {
            Err(())
        } else {
            Ok(())
        }
    }

    pub const fn reset_progress(&mut self) {
        self.progress = 0.0;
    }

    pub const fn progress(&self) -> f32 {
        self.progress
    }
}
