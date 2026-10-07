//! Used to describe the type of a `Segment`.
//! Does not include an `ExitLine` as this is not needed.

use crate::*;

#[derive(Component)]
pub(crate) struct EntryLine {
    flow_rates: FlowRates,
    /// The cached sum of the `Frequency` values in `self.flow_rates`.
    total_flow_rate: Frequency,
}

impl EntryLine {
    pub fn new(flow_rates: FlowRates) -> Self {
        let total_flow_rate = flow_rates.iter().map(|(_, &frequency)| frequency).sum();
        EntryLine {
            flow_rates,
            total_flow_rate,
        }
    }

    pub const fn flow_rates(&self) -> &FlowRates {
        &self.flow_rates
    }

    pub const fn total_flow_rate(&self) -> Frequency {
        self.total_flow_rate
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
