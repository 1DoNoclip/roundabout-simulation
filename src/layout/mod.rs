use crate::*;

pub(crate) mod assembly;
pub(crate) mod components;
pub(crate) mod conflict_points;
pub(crate) mod curve;
pub(crate) mod geometry;
pub(crate) mod settings;
pub(crate) mod yield_points;

pub(crate) use assembly::*;
pub(crate) use components::*;
pub(crate) use conflict_points::*;
pub(crate) use curve::*;
pub(crate) use geometry::*;
pub(crate) use settings::*;
pub(crate) use yield_points::*;

pub(crate) struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RegenerateLayout>()
            .add_plugins((
                AssemblyPlugin,
                ComponentsPlugin,
                ConflictPointsPlugin,
                CurvePlugin,
                GeometryPlugin,
                SettingsPlugin,
                YieldPointsPlugin,
            ))
            .add_systems(Startup, initialize_generation)
            .add_systems(Update, regenerate_layout);
    }
}

#[derive(Message)]
pub(crate) struct RegenerateLayout;

/// Creates a `RegenerateLayout` message.
fn initialize_generation(mut writer: MessageWriter<RegenerateLayout>) {
    writer.write(RegenerateLayout);
}

pub(crate) fn get_arm_flow_rates(
    arm_blueprints: &[ArmBlueprint],
    number_of_arms: usize,
    number_of_lanes: usize,
    roundabout_topology: &RoundaboutTopology,
    arm: Arm,
) -> HashMap<usize, FlowRates> {
    let mut arm_flow_rates = HashMap::<usize, FlowRates>::new();
    for other_arm_index in 0..number_of_arms {
        let other_arm = Arm::new(other_arm_index, arm_blueprints[other_arm_index].angle());
        let other_arm_id = roundabout_topology.get_arm_id_at(other_arm_index);
        let lane_index = select_lane_index(&arm, &other_arm, number_of_arms, number_of_lanes);
        arm_flow_rates
            .entry(lane_index)
            .or_insert_with(FlowRates::new)
            .insert(other_arm_id, Frequency::new::<per_hour>(800.0));
    }
    arm_flow_rates
}

fn regenerate_layout(mut commands: Commands, mut reader: MessageReader<RegenerateLayout>) {
    // If one or more RegenerateLayout messages have been created.
    // If more than 1 messages have been created, only regenerate once.
    if reader.is_empty().not() {
        reader.clear();
        commands.queue(move |world: &mut World| {
            info!("RoundaboutBlueprint was created/changed. Running layout generation pipeline.");
            world.run_system_cached(assemble_roundabout).unwrap();
            world.flush();
            world
                .run_system_cached(RoundaboutConflictPoints::generate)
                .unwrap();
            world
                .run_system_cached(RoundaboutYieldPoints::generate)
                .unwrap();
            info!("Roundabout layout and conflict points successfully updated.");
        });
    }
}
