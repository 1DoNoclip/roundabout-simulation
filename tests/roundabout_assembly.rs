use roundabout_simulation::*;

#[test]
fn test_assemble_roundabout_spawns_correct_topology() {
    let mut app = App::new();

    app.insert_resource(IntersectionBlueprint {
        number_of_lanes: 2,
        deflection_radius: 15.0,
        speed_limit: Speed::from_miles_per_hour(30.0).expect("failed to create Speed"),
        arms: vec![
            ArmBlueprint { angle: Rot2::degrees(0.0) },
            ArmBlueprint { angle: Rot2::degrees(120.0) },
            ArmBlueprint { angle: Rot2::degrees(240.0) },
        ],
    });
    app.insert_resource(RoundaboutCircleBlueprint { radius: 20.0 });

    app.add_systems(Update, assemble_roundabout);

    // First update enqueues the commands.
    app.update();
    // Flush forces Bevy to apply all queued commands to the World immediately.
    app.world_mut().flush();

    let mut query = app.world_mut().query::<&Segment>();
    assert!(query.iter(app.world()).count() > 0);
}
