use bevy::platform::collections::HashMap;
use crate::*;

pub(super) struct YieldPointPlugin;

impl Plugin for YieldPointPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RoundaboutYieldPoints>()
            .register_type::<YieldPoint>()
            .register_type::<YieldPointIndex>();
    }
}

pub(crate) struct RoundaboutYieldPoints {
    points: HashMap<YieldPointIndex, YieldPoint>,
}

pub(crate) struct YieldPoint {

}

pub(crate) struct YieldPointIndex {

}
