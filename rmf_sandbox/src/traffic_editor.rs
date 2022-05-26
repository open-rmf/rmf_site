use crate::camera_controls::{CameraControls, ProjectionMode};
use crate::lane::Lane;
use crate::measurement::Measurement;
use crate::model::Model;
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
use bevy_mod_picking::{
    DefaultPickingPlugins, PickableBundle, PickingBlocker, PickingCamera, PickingCameraBundle,
};

trait Editable: Sync + Send + Clone {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool;
}

impl Editable for Vertex {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("vertex").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            changed = ui.text_edit_singleline(&mut self.name).changed() || changed;
            ui.end_row();

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
        });

        changed
    }
}

impl Editable for Lane {
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
            changed = ui.text_edit_singleline(&mut self.model_name).lost_focus() || changed;
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
    Model(Entity),
}

enum EditorData {
    Lane(Lane),
    Vertex(Vertex),
    Measurement(Measurement),
    Wall(Wall),
    Model(Model),
}

struct SelectedEditable(Entity, EditorData);

#[derive(SystemParam)]
struct EditorPanel<'w, 's> {
    selected: ResMut<'w, Option<SelectedEditable>>,
    q_lane: Query<'w, 's, &'static mut Lane>,
    q_vertex: Query<'w, 's, &'static mut Vertex>,
    q_measurement: Query<'w, 's, &'static mut Measurement>,
    q_wall: Query<'w, 's, &'static mut Wall>,
    q_model: Query<'w, 's, &'static mut Model>,
}

impl<'w, 's> EditorPanel<'w, 's> {
    fn draw(&mut self, egui_ctx: &mut EguiContext) {
        fn commit_changes<E: Editable + Component>(
            q: &mut Query<&mut E>,
            target_entity: Entity,
            updated: &E,
        ) {
            match q.get_mut(target_entity) {
                Ok(mut e) => {
                    *e = updated.clone();
                }
                Err(err) => {
                    println!("ERROR: {err}");
                }
            }
        }

        egui::SidePanel::right("editor_panel")
            .resizable(false)
            .min_width(200.)
            .show(egui_ctx.ctx_mut(), |ui| {
                let selected = match *self.selected {
                    Some(ref mut selected) => selected,
                    None => {
                        ui.add_sized(ui.available_size(), egui::Label::new("No object selected"));
                        return;
                    }
                };

                let title = match &selected.1 {
                    EditorData::Lane(_) => "Lane",
                    EditorData::Vertex(_) => "Vertex",
                    EditorData::Measurement(_) => "Measurement",
                    EditorData::Wall(_) => "Wall",
                    EditorData::Model(_) => "Model",
                };

                ui.heading(title);
                ui.separator();

                match &mut selected.1 {
                    EditorData::Lane(lane) => {
                        if lane.draw(ui) {
                            commit_changes(&mut self.q_lane, selected.0, lane);
                        }
                    }
                    EditorData::Vertex(vertex) => {
                        if vertex.draw(ui) {
                            commit_changes(&mut self.q_vertex, selected.0, vertex);
                        }
                    }
                    EditorData::Measurement(measurement) => {
                        if measurement.draw(ui) {
                            commit_changes(&mut self.q_measurement, selected.0, measurement);
                        }
                    }
                    EditorData::Wall(wall) => {
                        if wall.draw(ui) {
                            commit_changes(&mut self.q_wall, selected.0, wall);
                        }
                    }
                    EditorData::Model(model) => {
                        if model.draw(ui) {
                            commit_changes(&mut self.q_model, selected.0, model);
                        }
                    }
                };
            });
    }
}

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut _exit: EventWriter<AppExit>,
    _thread_pool: Res<AsyncComputeTaskPool>,
    mut app_state: ResMut<State<AppState>>,
    mut editor: EditorPanel,
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

    editor.draw(&mut egui_context);
}

fn on_startup(mut commands: Commands) {
    commands
        .spawn()
        .insert(PickingBlocker)
        .insert(Interaction::default());
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<SiteMap>();
    commands.init_resource::<Option<SelectedEditable>>();
}

fn maintain_inspected_entities(
    editables: Query<(Entity, &Interaction, &EditableTag), Changed<Interaction>>,
    mut selected: ResMut<Option<SelectedEditable>>,
    q_lane: Query<&Lane>,
    q_vertex: Query<&Vertex>,
    q_measurement: Query<&Measurement>,
    q_wall: Query<&Wall>,
    q_model: Query<&Model>,
) {
    let clicked = editables.iter().find(|(_, i, _)| match i {
        Interaction::Clicked => true,
        _ => false,
    });
    let (e, _, tag) = match clicked {
        Some(clicked) => clicked,
        None => return,
    };
    let try_selected = match tag {
        // Clone and draw an inspectable so as to avoid change detection in bevy.
        // This also allows us to commit changes only when needed, e.g. commit only
        // when the user press "enter" when editing a text field.
        //
        // Bevy change detection works by implementing the dereference operator to mark something
        // as changed, this cause the change detection to trigger even if there are no writes to
        // it. Egui on the other hand requires data to be mutable, so passing a component directly
        // to egui will cause change detection to trigger every frame.
        EditableTag::Lane => match q_lane.get(e) {
            Ok(lane) => Ok(SelectedEditable(e, EditorData::Lane(lane.clone()))),
            Err(err) => Err(err),
        },
        EditableTag::Vertex => match q_vertex.get(e) {
            Ok(vertex) => Ok(SelectedEditable(e, EditorData::Vertex(vertex.clone()))),
            Err(err) => Err(err),
        },
        EditableTag::Measurement => match q_measurement.get(e) {
            Ok(measurement) => Ok(SelectedEditable(
                e,
                EditorData::Measurement(measurement.clone()),
            )),
            Err(err) => Err(err),
        },
        EditableTag::Wall => match q_wall.get(e) {
            Ok(wall) => Ok(SelectedEditable(e, EditorData::Wall(wall.clone()))),
            Err(err) => Err(err),
        },
        EditableTag::Model(model_entity) => match q_model.get(*model_entity) {
            Ok(model) => Ok(SelectedEditable(
                *model_entity,
                EditorData::Model(model.clone()),
            )),
            Err(err) => Err(err),
        },
    };

    *selected = match try_selected {
        Ok(selected) => Some(selected),
        Err(err) => {
            println!("{err}");
            None
        }
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
    models: Query<Entity, With<Model>>,
    meshes: Query<Entity, Added<Handle<Mesh>>>,
    parent: Query<&Parent>,
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

    // bevy_mod_picking only works on entities with meshes, the models are spawned with
    // child scenes so making the model entity pickable will not work. We need to check
    // all meshes added and go up the hierarchy to find if the mesh is part of a model.
    for mesh_entity in meshes.iter() {
        // go up the hierarchy tree until the root, trying to find if a mesh is part of a model.
        let mut pe = parent.get(mesh_entity);
        while let Ok(Parent(e)) = pe {
            // check if this entity is a model, if so, make it pickable.
            if let Ok(model_entity) = models.get(*e) {
                commands
                    .entity(mesh_entity)
                    .insert_bundle(PickableBundle::default())
                    .insert(EditableTag::Model(model_entity));
                break;
            }
            pe = parent.get(*e);
        }
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
