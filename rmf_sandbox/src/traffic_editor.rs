use crate::camera_controls::{CameraControls, ProjectionMode};
use crate::lane::Lane;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::site_map::SiteMap;
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
use bevy_mod_picking::{
    DefaultPickingPlugins, PickableBundle, PickingBlocker, PickingCamera, PickingCameraBundle,
};

trait Editable: Sync + Send + Clone {
    fn title() -> &'static str;
    fn draw(&mut self, ui: &mut egui::Ui) -> bool;
}

impl Editable for Vertex {
    fn title() -> &'static str {
        "Vertex"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("vertex").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            changed = ui.text_edit_singleline(&mut self.name).changed() || changed;
            ui.end_row();

            ui.label("X (Meters)");
            changed = ui
                .add(egui::DragValue::new(&mut self.x).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Y (Meters)");
            changed = ui
                .add(egui::DragValue::new(&mut self.y).speed(0.1))
                .changed()
                || changed;
            ui.end_row();
        });

        changed
    }
}

impl Editable for Lane {
    fn title() -> &'static str {
        "Lane"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("lane").num_columns(2).show(ui, |ui| {
            ui.label("Start");
            changed = ui.add(egui::DragValue::new(&mut self.start)).changed() || changed;
            ui.end_row();

            ui.label("End");
            changed = ui.add(egui::DragValue::new(&mut self.end)).changed() || changed;
            ui.end_row();
        });

        changed
    }
}

impl Editable for Measurement {
    fn title() -> &'static str {
        "Measurement"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("measurement")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Start");
                changed = ui.add(egui::DragValue::new(&mut self.start)).changed() || changed;
                ui.end_row();

                ui.label("End");
                changed = ui.add(egui::DragValue::new(&mut self.end)).changed() || changed;
                ui.end_row();

                // TODO: Remove this field once we support new cartesian format. Doing so removes
                // the ambiguity between the actual distance (from calculations) and the distance defined
                // in the file.
                ui.label("Distance");
                changed = ui
                    .add(egui::DragValue::new(&mut self.distance).speed(0.1))
                    .changed()
                    || changed;
                ui.end_row();
            });

        changed
    }
}

impl Editable for Wall {
    fn title() -> &'static str {
        "Wall"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("wall").num_columns(2).show(ui, |ui| {
            ui.label("Start");
            changed = ui.add(egui::DragValue::new(&mut self.start)).changed() || changed;
            ui.end_row();

            ui.label("End");
            changed = ui.add(egui::DragValue::new(&mut self.end)).changed() || changed;
            ui.end_row();

            ui.label("Height");
            changed = ui
                .add(egui::DragValue::new(&mut self.height).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Texture");
            changed = ui.text_edit_singleline(&mut self.texture_name).changed() || changed;
            ui.end_row();
        });

        changed
    }
}

impl Editable for Model {
    fn title() -> &'static str {
        "Model"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("model").num_columns(2).show(ui, |ui| {
            ui.label("X");
            changed = ui
                .add(egui::DragValue::new(&mut self.x).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Y");
            changed = ui
                .add(egui::DragValue::new(&mut self.y).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Yaw");
            changed = ui
                .add(egui::DragValue::new(&mut self.yaw).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Z Offset");
            changed = ui
                .add(egui::DragValue::new(&mut self.z_offset).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Model");
            changed = ui.text_edit_singleline(&mut self.model_name).changed() || changed;
            ui.end_row();
        });

        changed
    }
}

#[derive(Component)]
enum EditableTag {
    Lane,
    Vertex,
    Measurement,
    Wall,
    Model,
}

struct SelectedEditable(Entity);

struct EditorWindow;

/// Clone and draw an inspectable so as to avoid change detection in bevy.
///
/// Bevy change detection works by implementing the dereference operator to mark something
/// as changed, this cause the change detection to trigger even if there are no writes to
/// it. Egui on the other hand requires data to be mutable, so passing a component directly
/// to egui will cause change detection to trigger every frame.
fn clone_and_draw<E: Editable>(ui: &mut egui::Ui, mut editable: Mut<E>) {
    let mut ui_data = editable.clone();
    if ui_data.draw(ui) {
        *editable = ui_data.clone();
    }
}

impl EditorWindow {
    fn draw<E: Editable>(egui_ctx: &mut EguiContext, e: Mut<E>) {
        egui::Window::new(E::title())
            .id(egui::Id::new("inspector"))
            .collapsible(false)
            .resizable(false)
            .show(egui_ctx.ctx_mut(), |ui| {
                clone_and_draw(ui, e);
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
    selected: ResMut<Option<SelectedEditable>>,
    q_editable_tag: Query<&EditableTag>,
    mut q_editable: Query<(
        Option<&mut Lane>,
        Option<&mut Measurement>,
        Option<&mut Vertex>,
        Option<&mut Wall>,
        Option<&mut Model>,
    )>,
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

    match &*selected {
        Some(selected) => {
            let tag = q_editable_tag.get(selected.0).unwrap();
            match tag {
                EditableTag::Lane => EditorWindow::draw(
                    &mut egui_context,
                    q_editable.get_component_mut::<Lane>(selected.0).unwrap(),
                ),
                EditableTag::Vertex => EditorWindow::draw(
                    &mut egui_context,
                    q_editable.get_component_mut::<Vertex>(selected.0).unwrap(),
                ),
                EditableTag::Measurement => EditorWindow::draw(
                    &mut egui_context,
                    q_editable
                        .get_component_mut::<Measurement>(selected.0)
                        .unwrap(),
                ),
                EditableTag::Wall => EditorWindow::draw(
                    &mut egui_context,
                    q_editable.get_component_mut::<Wall>(selected.0).unwrap(),
                ),
                EditableTag::Model => EditorWindow::draw(
                    &mut egui_context,
                    q_editable.get_component_mut::<Model>(selected.0).unwrap(),
                ),
            };
        }
        None => (),
    };
}

fn on_startup(mut commands: Commands) {
    commands
        .spawn()
        .insert(PickingBlocker)
        .insert(Interaction::default());
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<SiteMap>();
}

fn maintain_inspected_entities(
    editables: Query<(Entity, &Interaction), (Changed<Interaction>, With<EditableTag>)>,
    mut selected: ResMut<Option<SelectedEditable>>,
) {
    editables.iter().any(|(e, i)| match i {
        Interaction::Clicked => {
            *selected = Some(SelectedEditable(e.clone()));
            true
        }
        _ => false,
    });
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
    models: Query<Entity, Added<Model>>,
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Query<&mut Interaction, With<PickingBlocker>>,
) {
    // Go through all the entities spawned by site map and make them response to
    // the inspector window.
    for entity in lanes.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(EditableTag::Lane);
    }
    for entity in vertices.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(EditableTag::Vertex);
    }
    for entity in walls.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(EditableTag::Wall);
    }
    for entity in measurements.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(EditableTag::Measurement);
    }
    // FIXME: Picking is not working for models.
    for entity in models.iter() {
        commands
            .entity(entity)
            .insert_bundle(PickableBundle::default())
            .insert(EditableTag::Model);
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
            .init_resource::<Option<SelectedEditable>>()
            // .add_plugin(InspectorPlugin::<Inspector>::new())
            // .register_inspectable::<Lane>()
            .add_startup_system(on_startup);

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
