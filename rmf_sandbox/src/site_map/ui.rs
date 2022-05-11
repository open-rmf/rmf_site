use crate::camera_controls::{CameraControls, ProjectionMode};
use crate::lane::Lane;
use crate::measurement::Measurement;
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::AppState;
use bevy::{
    app::AppExit,
    prelude::*,
    render::camera::{ActiveCamera, Camera3d},
    tasks::AsyncComputeTaskPool,
};
use bevy_egui::{egui, EguiContext};
use bevy_inspector_egui::plugin::InspectorWindows;
use bevy_inspector_egui::{Inspectable, InspectorPlugin, RegisterInspectable};
use bevy_mod_picking::{DefaultPickingPlugins, PickingBlocker, PickingCamera, PickingCameraBundle};

#[derive(Inspectable, Default)]
struct Inspector {
    #[inspectable(deletable = false)]
    active: Option<Editable>,
}

#[derive(Inspectable, Component, Clone)]
pub enum Editable {
    Lane(Lane),
    Measurement(Measurement),
    Vertex(Vertex),
    Wall(Wall),
}

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    mut _commands: Commands,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut _exit: EventWriter<AppExit>,
    _thread_pool: Res<AsyncComputeTaskPool>,
    mut app_state: ResMut<State<AppState>>,
    mut inspector_windows: ResMut<InspectorWindows>,
) {
    let mut controls = query.single_mut();
    egui::TopBottomPanel::top("top").show(egui_context.ctx_mut(), |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("Return to main menu").clicked() {
                    println!("Returning to main menu");
                    app_state.set(AppState::MainMenu).unwrap();
                }
                ui.separator();
                if ui
                    .add(egui::SelectableLabel::new(
                        controls.mode == ProjectionMode::Orthographic,
                        "2D",
                    ))
                    .clicked()
                {
                    controls.set_mode(ProjectionMode::Orthographic);
                    active_camera_3d.set(controls.orthographic_camera_entity);
                }
                if ui
                    .add(egui::SelectableLabel::new(
                        controls.mode == ProjectionMode::Perspective,
                        "3D",
                    ))
                    .clicked()
                {
                    controls.set_mode(ProjectionMode::Perspective);
                    active_camera_3d.set(controls.perspective_camera_entity);
                }
                if ui
                    .add(egui::SelectableLabel::new(
                        inspector_windows.window_data_mut::<Inspector>().visible,
                        "Inspector",
                    ))
                    .clicked()
                {
                    inspector_windows.window_data_mut::<Inspector>().visible =
                        !inspector_windows.window_data_mut::<Inspector>().visible;
                }
            });
        });
    });
}

fn on_startup(mut commands: Commands, mut inspector_windows: ResMut<InspectorWindows>) {
    commands
        .spawn()
        .insert(PickingBlocker)
        .insert(Interaction::default());
    inspector_windows.window_data_mut::<Inspector>().visible = false;
}

fn on_enter(mut inspector_windows: ResMut<InspectorWindows>) {
    inspector_windows.window_data_mut::<Inspector>().visible = true;
}

fn on_exit(mut inspector_windows: ResMut<InspectorWindows>) {
    inspector_windows.window_data_mut::<Inspector>().visible = false;
}

fn maintain_inspected_entities(
    mut inspector: ResMut<Inspector>,
    editables: Query<(&Editable, &Interaction), Changed<Interaction>>,
) {
    let selected = editables.iter().find_map(|(e, i)| match i {
        Interaction::Clicked => Some(e),
        _ => None,
    });
    if let Some(selected) = selected {
        inspector.active = Some(selected.clone())
    }
}

fn update_picking_cam(
    mut commands: Commands,
    opt_active_camera: Option<Res<ActiveCamera<Camera3d>>>,
    picking_cams: Query<Entity, With<PickingCamera>>,
) {
    let active_camera = match opt_active_camera {
        Some(cam) => cam,
        None => return,
    };
    if active_camera.is_changed() {
        match active_camera.get() {
            Some(active_cam) => {
                // remove all previous picking cameras
                for cam in picking_cams.iter() {
                    commands.entity(cam).remove_bundle::<PickingCameraBundle>();
                }
                commands
                    .entity(active_cam)
                    .insert_bundle(PickingCameraBundle::default());
            }
            None => (),
        }
    }
}

/// Stops picking when egui is in focus.
/// This creates a dummy PickingBlocker and make it "Clicked" whenever egui is in focus.
///
/// Normally bevy_mod_picking automatically stops when
/// a bevy ui node is in focus, but bevy_egui does not use bevy ui node.
fn enable_picking(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Query<&mut Interaction, With<PickingBlocker>>,
) {
    let egui_ctx = egui_context.ctx_mut();
    let enable = !egui_ctx.wants_pointer_input() && !egui_ctx.wants_keyboard_input();

    let mut blocker = picking_blocker.single_mut();
    if enable {
        *blocker = Interaction::None;
    } else {
        *blocker = Interaction::Clicked;
    }
}

#[derive(Default)]
pub struct SiteMapUIPlugin;

impl Plugin for SiteMapUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(InspectorPlugin::<Inspector>::new())
            .register_inspectable::<Lane>()
            .add_startup_system(on_startup);

        app.add_system_set(SystemSet::on_enter(AppState::SiteMap).with_system(on_enter));

        app.add_system_set(
            SystemSet::on_update(AppState::SiteMap)
                .with_system(egui_ui)
                .with_system(update_picking_cam)
                .with_system(enable_picking)
                .with_system(maintain_inspected_entities),
        );

        app.add_system_set(SystemSet::on_exit(AppState::SiteMap).with_system(on_exit));
    }
}
