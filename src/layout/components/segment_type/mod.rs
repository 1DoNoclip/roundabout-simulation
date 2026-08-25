//! Used to describe the type of a `Segment`.
//! Is not currently exhaustive for all segment types because they are not needed yet.

use crate::*;

#[derive(Component)]
pub(crate) struct EntryLine;

#[derive(Component)]
pub(crate) struct EntryDeflection;

#[derive(Component)]
pub(crate) struct InterArmSector;

#[derive(Component)]
pub(crate) struct IntraArmSector;
