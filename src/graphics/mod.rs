use crate::*;

pub(crate) struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SegmentInspectorState>()
            .register_type::<SegmentInspectorState>()
            .init_gizmo_group::<VehicleGizmos>()
            .add_systems(Startup, setup_gizmos)
            .add_systems(
                Update,
                (
                    draw_layout,
                    draw_conflict_points,
                    draw_yield_points,
                    draw_vehicles,
                    cycle_segment_gizmos,
                )
                    .chain(),
            );
    }
}

/// Displays "RENDERING DISABLED" on the screen when `--no-render` is used.
pub(super) fn setup_no_render_overlay(mut commands: Commands) {
    const MESSAGE: &str = "RENDERING DISABLED";
    const LENGTH: usize = MESSAGE.len();

    commands.spawn((
        Text::new(MESSAGE),
        TextFont {
            // Use 75% of the screen width.
            font_size: FontSize::Vw(75.0 / LENGTH as f32),
            ..default()
        },
        TextColor(Color::srgb_u8(200, 50, 50)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            // The width of the box.
            width: Val::Percent(75.0),
            left: Val::Percent(12.5),
            top: Val::Percent(55.0),
            ..default()
        },
    ));
}

fn setup_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<VehicleGizmos>();
    config.line.width = 4.0;
}

fn draw_layout(mut gizmos: Gizmos, segments: Query<&Segment>) {
    segments
        .iter()
        .for_each(|segment| draw_segment(&mut gizmos, segment, None));
}

fn draw_conflict_points(mut gizmos: Gizmos, conflict_points: Res<RoundaboutConflictPoints>) {
    let conflict_points = conflict_points.points();
    for (_, conflict_point) in conflict_points {
        gizmos.circle_2d(
            conflict_point.location.xy(),
            0.75,
            Color::linear_rgb(0.0, 0.0, 255.0),
        );
    }
}

fn draw_yield_points(mut gizmos: Gizmos, yield_points: Res<RoundaboutYieldPoints>) {
    let yield_points = yield_points.points();
    for (_, yield_point) in yield_points {
        gizmos.circle_2d(
            yield_point.location.xy(),
            0.75,
            Color::linear_rgb(17.0, 200.0, 182.0),
        );
    }
}

/// Used to draw vehicles with a different line width.
#[derive(Default, GizmoConfigGroup, Reflect)]
struct VehicleGizmos;

fn draw_vehicles(
    mut gizmos: Gizmos<VehicleGizmos>,
    vehicles: Query<(&Kinematics, &Transform), With<Vehicle>>,
) {
    for (kinematics, transform) in vehicles.iter() {
        let direction = transform.rotation * Vec3::X;
        let length = kinematics.vehicle_length_metres();

        let start_position = transform.translation;
        let end_position = start_position + (direction * length);

        gizmos.line(
            start_position,
            end_position,
            Color::linear_rgb(255.0, 0.0, 255.0),
        );
    }
}

fn cycle_segment_gizmos(
    mut gizmos: Gizmos,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SegmentInspectorState>,
    segments: Query<&Segment>,
) {
    const HIGHLIGHT_COLORS: GizmoColors = GizmoColors::srgb_u8([0, 255, 0], [0, 255, 0]);

    let segment_list = segments.iter().collect::<Vec<_>>();
    if segment_list.is_empty() {
        return;
    }

    // Press Tab to cycle through segments.
    if keyboard.just_pressed(KeyCode::Tab) {
        state.selected_index = Some(match state.selected_index {
            Some(index) => (index + 1) % segment_list.len(),
            None => 0,
        });
    }

    // Render the currently selected segment in green.
    if let Some(index) = state.selected_index {
        if let Some(segment) = segment_list.get(index) {
            draw_segment(&mut gizmos, segment, Some(HIGHLIGHT_COLORS));
        } else {
            // Clamp to maximum if the inspector GUI has gone over limit.
            state.selected_index = Some(segment_list.len() - 1);
        }
    }
}

fn draw_segment(gizmos: &mut Gizmos, segment: &Segment, color_override: Option<GizmoColors>) {
    const NUMBER_OF_SAMPLES: usize = 100;

    let gizmo_colors =
        color_override.unwrap_or_else(|| GizmoColors::get_colors(segment.connection()));

    let mut previous_point = segment.position_at(0.0);
    for step in 1..=NUMBER_OF_SAMPLES {
        let time = step as f32 / NUMBER_OF_SAMPLES as f32;
        let current_point = segment.position_at(time);
        gizmos.line(previous_point, current_point, gizmo_colors.segment);
        previous_point = current_point;
    }

    // Small sphere marker at the segment end point.
    gizmos.sphere(
        Isometry3d::from_translation(previous_point),
        0.75,
        gizmo_colors.point,
    );
}

// The default value for Option is None.
#[derive(Default, Reflect, Resource)]
#[reflect(Resource)]
struct SegmentInspectorState {
    pub selected_index: Option<usize>,
}

struct GizmoColors {
    /// The color of the segment.
    segment: Color,
    /// The color of the end of the segment (a point placed at the end).
    point: Color,
}

impl GizmoColors {
    const fn srgb_u8(segment: [u8; 3], point: [u8; 3]) -> Self {
        GizmoColors {
            segment: Color::srgb_u8(segment[0], segment[1], segment[2]),
            point: Color::srgb_u8(point[0], point[1], point[2]),
        }
    }

    /// Uses a segment's connection type to determine the color of the segment and the end of the segment.
    const fn get_colors(connection: &Connection) -> GizmoColors {
        match connection {
            Connection::Merge { .. } => {
                // Yellow / dark yellow.
                GizmoColors::srgb_u8([200, 200, 46], [149, 149, 34])
            }
            Connection::Direct { .. } => {
                //| Connection::Diverge { .. } => {
                // White / grey.
                GizmoColors::srgb_u8([100, 100, 100], [100, 100, 100])
            }
            Connection::Diverge { .. } => {
                // Grey / grey.
                GizmoColors::srgb_u8([100, 100, 100], [100, 100, 100])
            }
            Connection::EndPoint { .. } => {
                // Red / dark.
                GizmoColors::srgb_u8([200, 20, 32], [161, 16, 25])
            }
        }
    }
}
