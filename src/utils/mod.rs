use crate::*;

pub(crate) mod units;

pub(crate) use units::*;

pub(super) struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UnitsPlugin);
    }
}
