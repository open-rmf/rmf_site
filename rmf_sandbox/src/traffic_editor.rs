use std::ops::DerefMut;

use crate::camera_controls::{CameraControls, ProjectionMode};
use crate::lane::Lane;
use crate::measurement::Measurement;
use crate::site_map::SiteMap;
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::AppState;
use bevy::ecs::system::SystemParam;
use bevy::{
    app::AppExit,
    prelude::*,
    render::camera::{ActiveCamera, Camera3d},
    tasks::AsyncComputeTaskPool,
};
use bevy_egui::{egui, EguiContext};
use bevy_inspector_egui::Inspectable;
use bevy_mod_picking::{
    DefaultPickingPlugins, PickableBundle, PickingBlocker, PickingCamera, PickingCameraBundle,
};

#[derive(Component, Clone)]
pub enum Editable {
    Lane(Entity),
    Measurement(Entity),
    Vertex(Entity),
    Wall(Entity),
}

impl Editable {
    fn get_title(&self) -> &'static str {
        match self {
            Editable::Lane(_) => "Lane",
            Editable::Measurement(_) => "Measurement",
            Editable::Vertex(_) => "Vertex",
            Editable::Wall(_) => "Wall",
        }
    }
}

#[derive(SystemParam)]
struct EditorWindow<'w, 's> {
    q: Query<
        'w,
        's,
        (
            Option<&'static mut Lane>,
            Option<&'static mut Measurement>,
            Option<&'static mut Vertex>,
            Option<&'static mut Wall>,
        ),
    >,
}

/// Clone and draw an inspectable so as to avoid change detection in bevy.
/// 
/// Bevy change detection works by implementing the dereference operator to mark something
/// as changed, this cause the change detection to trigger even if there are no writes to
/// it. Egui on the other hand requires data to be mutable, so passing a component directly
/// to egui will cause change detection to trigger every frame.
fn clone_and_draw<I: Inspectable + Clone>(ui: &mut egui::Ui, inspectable: &mut Mut<I>) {
    let mut inspector_context = bevy_inspector_egui::Context::new_shared(None);
    let mut ui_data = inspectable.clone();
    if ui_data.ui(ui, I::Attributes::default(), &mut inspector_context) {
        **inspectable = ui_data.clone();
    }
}

impl<'w, 's> EditorWindow<'w, 's> {
    fn draw(&mut self, egui_ctx: &mut EguiContext, e: &Editable) {
        egui::Window::new(e.get_title())
            .id(egui::Id::new("Inspector"))
            .collapsible(false)
            .show(egui_ctx.ctx_mut(), |ui| match e {
                Editable::Lane(e) => {
                    let mut lane = self.q.get_component_mut::<Lane>(*e).unwrap();
                    clone_and_draw(ui, &mut lane);
                }
                Editable::Measurement(e) => {
                    let mut measurement = self.q.get_component_mut::<Measurement>(*e).unwrap();
                    clone_and_draw(ui, &mut measurement);
                }
                Editable::Vertex(e) => {
                    let mut vertex = self.q.get_component_mut::<Vertex>(*e).unwrap();
                    clone_and_draw(ui, &mut vertex);
                }
                Editable::Wall(e) => {
                    let mut wall = self.q.get_component_mut::<Wall>(*e).unwrap();
                    clone_and_draw(ui, &mut wall);
                }
            });
    }
}

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    mut _commands: Commands,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut _exit: EventWriter<AppExit>,
    _thread_pool: Res<AsyncComputeTaskPool>,
    mut app_state: ResMut<State<AppState>>,
    mut selected: ResMut<Option<Editable>>,
    mut editor_window: EditorWindow,
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
            });
        });
    });

    if let Some(selected) = selected.deref_mut() {
        editor_window.draw(&mut *egui_context, selected);
    }
}

fn on_startup(mut commands: Commands) {
    commands
        .spawn()
        .insert(PickingBlocker)
        .insert(Interaction::default());
    // inspector_windows.window_data_mut::<Inspector>().visible = false;
}

fn on_enter() {
    // inspector_windows.window_data_mut::<Inspector>().visible = true;
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<SiteMap>();
    // inspector_windows.window_data_mut::<Inspector>().visible = false;
}

fn maintain_inspected_entities(
    // mut inspector: ResMut<Inspector>,
    editables: Query<(&Editable, &Interaction), Changed<Interaction>>,
    mut selected: ResMut<Option<Editable>>,
) {
    let clicked = editables.iter().find_map(|(e, i)| match i {
        Interaction::Clicked => Some(e.clone()),
        _ => None,
    });
    if let Some(clicked) = clicked {
        *selected = Some(clicked.clone())
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

fn enable_picking(
    mut commands: Commands,
    lanes: Query<Entity, Added<Lane>>,
    vertices: Query<Entity, Added<Vertex>>,
    measurements: Query<Entity, Added<Measurement>>,
    walls: Query<Entity, Added<Wall>>,
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Query<&mut Interaction, With<PickingBlocker>>,
) {
    // Go through all the entities spawned by site map and make them response to
    // the inspector window.
    for entity in lanes.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Lane(entity));
    }
    for entity in vertices.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Vertex(entity));
    }
    for entity in walls.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Wall(entity));
    }
    for entity in measurements.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Measurement(entity));
    }

    // Stops picking when egui is in focus.
    // This creates a dummy PickingBlocker and make it "Clicked" whenever egui is in focus.
    //
    // Normally bevy_mod_picking automatically stops when
    // a bevy ui node is in focus, but bevy_egui does not use bevy ui node.
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
pub struct TrafficEditorPlugin;

impl Plugin for TrafficEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .init_resource::<Option<Editable>>()
            // .add_plugin(InspectorPlugin::<Inspector>::new())
            // .register_inspectable::<Lane>()
            .add_startup_system(on_startup);

        app.add_system_set(SystemSet::on_enter(AppState::TrafficEditor).with_system(on_enter));

        app.add_system_set(
            SystemSet::on_update(AppState::TrafficEditor)
                .with_system(egui_ui)
                .with_system(update_picking_cam)
                .with_system(enable_picking)
                .with_system(maintain_inspected_entities),
        );

        app.add_system_set(SystemSet::on_exit(AppState::TrafficEditor).with_system(on_exit));
    }
}
