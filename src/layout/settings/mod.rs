use crate::*;

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ApplyUiSettings>()
            .insert_resource(MapSettings::default())
            .insert_resource(SimulationSettings::default());
    }
}

pub(crate) fn update_flow_rates(mut map_settings: ResMut<MapSettings>) {
    let number_of_lanes = map_settings.number_of_lanes();
    // The outer `HashMap` key is the entry arm index.
    // The `Vec` index is the entry lane index.
    // The inner `HashMap` key is the exit arm index.
    let all_flow_rates: HashMap<usize, Vec<HashMap<usize, Frequency>>> = HashMap::new();


    for (arm_index, arm_flow_rates) in all_flow_rates {
        // Overwrite the old flow rates with the new.
        map_settings.arms[arm_index].arm_flow_rates = arm_flow_rates;
    }

    // // The key of the outer `HashMap` is the lane index.
    // // The key of the inner `HashMap` / `EntityHashMap` becomes the exit arm ID.
    // let mut arm_flow_rates: HashMap<usize, FlowRates> =
    //     get_arm_flow_rates(arm_blueprints, number_of_arms, number_of_lanes, arm)
    //         .into_iter()
    //         .map(|(lane_index, inner_map)| {
    //             let lane_flow_rates: FlowRates = inner_map
    //                 .into_iter()
    //                 .map(|(exit_arm_index, flow_rate)| {
    //                     let exit_arm_id = roundabout_topology.get_arm_id_at(exit_arm_index);
    //                     (exit_arm_id, flow_rate)
    //                 })
    //                 .collect();
    //             (lane_index, lane_flow_rates)
    //         })
    //         .collect();
}

#[derive(Message)]
pub(crate) enum ApplyUiSettings {
    Map,
    UpdateFlowRates,
    SimulationPlayPause,
    SimulationSpeed,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum SpeedUnit {
    MeterPerSecond,
    MilePerHour,
}

#[derive(Clone, PartialEq, Resource)]
pub(crate) struct MapSettings {
    pub(crate) number_of_lanes: usize,
    pub(crate) speed_limit: Velocity,
    pub(crate) current_ui_speed_unit: SpeedUnit,
    pub(crate) radius: Length,
    pub(crate) deflection_radius: Length,
    pub(crate) arms: Vec<ArmSettings>,
}

impl MapSettings {
    /// Not a method so that it works before `Self` has been created.
    fn update_arm_flow_rates(arms: &mut [ArmSettings], number_of_lanes: usize) {
        // Flow rates of each lane in each arm.
        // HashMap's key is the arm index.
        // Inside Vec's index is the lane index.
        // Inside HashMap's key is the exit arm index.
        let mut all_flow_rates: HashMap<usize, Vec<HashMap<usize, Frequency>>> = HashMap::new();
        let number_of_arms = arms.len();

        for (index, arm_settings) in arms.iter().enumerate() {
            let arm = Arm::new(index, arm_settings.angle());
            let mut arm_flow_rates: Vec<HashMap<usize, Frequency>> =
                core::iter::repeat_with(HashMap::new)
                    .take(number_of_lanes)
                    .collect();
            for (other_index, other_arm_settings) in arms.iter().enumerate() {
                let other_arm = Arm::new(other_index, other_arm_settings.angle());
                let lane_index =
                    select_lane_index(&arm, &other_arm, number_of_arms, number_of_lanes);
                arm_flow_rates[lane_index].insert(other_index, Frequency::new::<per_hour>(800.0));
            }

            all_flow_rates.insert(index, arm_flow_rates);
        }

        // Insert the calculated flow rates into each `ArmSettings`.
        for (index, arm_settings) in arms.iter_mut().enumerate() {
            if let Some(arm_flow_rates) = all_flow_rates.remove(&index) {
                arm_settings.arm_flow_rates = arm_flow_rates;
            } else {
                warn!("No flow rates found for arm_settings with index {index}");
            }
        }
    }

    pub const fn number_of_lanes(&self) -> usize {
        self.number_of_lanes
    }

    pub const fn speed_limit(&self) -> Velocity {
        self.speed_limit
    }

    pub const fn radius(&self) -> Length {
        self.radius
    }

    pub const fn deflection_radius(&self) -> Length {
        self.deflection_radius
    }

    pub fn arms(&self) -> &[ArmSettings] {
        &self.arms
    }
}

impl Default for MapSettings {
    fn default() -> Self {
        let number_of_lanes = 2;

        let mut arms = vec![
            ArmSettings::new(Rot2::degrees(0.0), 1_000, None, Vec::new()),
            ArmSettings::new(Rot2::degrees(-90.0), 1_000, None, Vec::new()),
            ArmSettings::new(Rot2::degrees(-180.0), 1_000, None, Vec::new()),
            ArmSettings::new(Rot2::degrees(-270.0), 1_000, None, Vec::new()),
        ];

        Self::update_arm_flow_rates(&mut arms, number_of_lanes);

        MapSettings {
            number_of_lanes,
            speed_limit: Velocity::new::<mile_per_hour>(30.0),
            current_ui_speed_unit: SpeedUnit::MilePerHour,
            radius: Length::new::<meter>(30.0),
            deflection_radius: Length::new::<meter>(12.5),
            arms,
        }
    }
}

#[derive(Resource)]
pub(crate) struct SimulationSettings {
    pub(crate) paused: bool,
    pub(crate) time_speed_factor: f32,
}

impl SimulationSettings {
    pub const fn paused(&self) -> bool {
        self.paused
    }

    pub(crate) const fn pause(&mut self) {
        self.paused = true;
    }

    pub(crate) const fn unpause(&mut self) {
        self.paused = false;
    }

    pub const fn time_speed_factor(&self) -> f32 {
        self.time_speed_factor
    }
}

impl Default for SimulationSettings {
    fn default() -> Self {
        SimulationSettings {
            paused: false,
            time_speed_factor: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArmSettings {
    pub(crate) angle: Rot2,
    vehicles_per_hour: u32,
    pub(crate) speed_limit_override: Option<Speed>,
    /// Each `Vec` index is a lane (index 0 is the inner lane).
    /// The `HashMap` key is the exit arm index.
    pub(crate) arm_flow_rates: Vec<HashMap<usize, Frequency>>,
}

impl ArmSettings {
    const fn new(
        angle: Rot2,
        vehicles_per_hour: u32,
        speed_limit_override: Option<Speed>,
        arm_flow_rates: Vec<HashMap<usize, Frequency>>,
    ) -> Self {
        ArmSettings {
            angle,
            vehicles_per_hour,
            speed_limit_override,
            arm_flow_rates,
        }
    }

    pub const fn angle(&self) -> Rot2 {
        self.angle
    }

    pub const fn vehicles_per_hour(&self) -> u32 {
        self.vehicles_per_hour
    }

    pub const fn speed_limit_override(&self) -> Option<Speed> {
        self.speed_limit_override
    }
}
