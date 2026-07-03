use crate::*;

#[derive(Component, Debug, Reflect)]
pub struct SpeedLimit {
    limit: f32,
}

impl SpeedLimit {
    pub fn new(limit: f32) -> Option<Self> {
        if limit < 0.0 || limit.is_nan() {
            None
        } else {
            Some(SpeedLimit { limit })
        }
    }

    pub fn limit(&self) -> f32 {
        self.limit
    }
}
