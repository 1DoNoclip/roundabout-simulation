//! Used to describe the type of a `Segment`.
//! Does not include an `ExitLine` as this is not needed.

use crate::*;

#[derive(Component)]
pub(crate) struct EntryLine {
    flow_rates: EntityHashMap<Frequency>,
}

impl EntryLine {
    pub const fn new(flow_rates: EntityHashMap<Frequency>) -> Self {
        EntryLine { flow_rates }
    }
}

#[derive(Component)]
pub(crate) struct EntryDeflection;

#[derive(Component)]
pub(crate) struct InterArmSector;

#[derive(Component)]
pub(crate) struct IntraArmSector;

#[derive(Component)]
pub(crate) struct ExitLine;

#[derive(Component)]
pub(crate) struct ExitDeflection;
