//! Used to describe the type of a `Segment`.
//! Does not include an `ExitLine` as this is not needed.

use crate::*;

#[derive(Component)]
pub(crate) struct EntryLine {
    adjusted_arrival_time: Frequency,
    headway_distribution: NormalDistr,
}

impl EntryLine {
    pub const fn new(adjusted_arrival_time: Frequency, headway_distribution: NormalDistr) -> Self {
        EntryLine {
            adjusted_arrival_time,
            headway_distribution,
        }
    }

    pub const fn adjusted_arrival_time(&self) -> Frequency {
        self.adjusted_arrival_time
    }

    pub const fn headway_distribution(&self) -> NormalDistr {
        self.headway_distribution
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
