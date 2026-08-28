//! Defines yield points for entering vehicles.

use crate::*;
use bevy::platform::collections::HashMap;

pub(super) struct YieldPointsPlugin;

impl Plugin for YieldPointsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RoundaboutYieldPoints>()
            .register_type::<YieldPoint>()
            .register_type::<YieldPointIndex>()
            .insert_resource(RoundaboutYieldPoints::default());
    }
}

#[derive(Default, Reflect, Resource)]
pub(crate) struct RoundaboutYieldPoints {
    points: HashMap<YieldPointIndex, YieldPoint>,
}

#[derive(Clone, Copy, Debug, Reflect)]
pub(crate) struct YieldPoint {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
pub(crate) struct YieldPointIndex {}
