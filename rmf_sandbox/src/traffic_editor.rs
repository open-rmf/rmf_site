use std::path::PathBuf;

use crate::basic_components;
use crate::building_map::BuildingMap;
use crate::camera_controls::{CameraControls, ProjectionMode};
use crate::door::{Door, DoorType, DOOR_TYPES};
use crate::floor::Floor;
use crate::lane::Lane;
use crate::lift::Lift;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::save_load::SaveMap;
use crate::site_map::{SiteMapCurrentLevel, SiteMapLabel, SiteMapState};
use crate::spawner::Spawner;
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::widgets::TextEditJson;
use crate::{AppState, OpenedMapFile};
use bevy::ecs::system::SystemParam;
use bevy::{
    app::AppExit,
    prelude::*,
    render::camera::{ActiveCamera, Camera3d},
    tasks::AsyncComputeTaskPool,
};
use bevy_egui::{egui, EguiContext};
use bevy_mod_picking::{
    DefaultHighlighting, DefaultPickingPlugins, PickableBundle, PickingBlocker, PickingCamera,
    PickingCameraBundle, Selection, StandardMaterialHighlight,
};

trait Editable: Sync + Send + Clone {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool;
}

impl Editable for Vertex {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("vertex").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            changed = ui.text_edit_singleline(&mut self.3).changed() || changed;
            ui.end_row();

            ui.label("X");
            changed = ui
                .add(egui::DragValue::new(&mut self.0).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Y");
            changed = ui
                .add(egui::DragValue::new(&mut self.1).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Z");
            changed = ui
                .add(egui::DragValue::new(&mut self.2).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Charger");
            changed = ui.checkbox(&mut self.4.is_charger, "").changed() || changed;
            ui.end_row();

            ui.label("Holding Point");
            changed = ui.checkbox(&mut self.4.is_holding_point, "").changed() || changed;
            ui.end_row();

            ui.label("Parking Spot");
            changed = ui.checkbox(&mut self.4.is_parking_spot, "").changed() || changed;
            ui.end_row();

            ui.label("Spawn Robot");
            changed = ui
                .text_edit_singleline(&mut *self.4.spawn_robot_name)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Spawn Robot Type");
            changed = ui
                .text_edit_singleline(&mut *self.4.spawn_robot_type)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Dropoff Ingestor");
            changed = ui
                .text_edit_singleline(&mut *self.4.dropoff_ingestor)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Pickup Dispenser");
            changed = ui
                .text_edit_singleline(&mut *self.4.pickup_dispenser)
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
            changed = ui.add(egui::DragValue::new(&mut self.0)).changed() || changed;
            ui.end_row();

            ui.label("End");
            changed = ui.add(egui::DragValue::new(&mut self.1)).changed() || changed;
            ui.end_row();

            ui.label("Bidirectional");
            changed = ui.checkbox(&mut self.2.bidirectional, "").changed() || changed;
            ui.end_row();

            ui.label("Graph");
            changed = ui
                .add(egui::DragValue::new(&mut self.2.graph_idx))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Orientation");
            changed = ui.text_edit_singleline(&mut *self.2.orientation).changed() || changed;
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
                changed = ui.add(egui::DragValue::new(&mut self.0)).changed() || changed;
                ui.end_row();

                ui.label("End");
                changed = ui.add(egui::DragValue::new(&mut self.1)).changed() || changed;
                ui.end_row();

                // TODO: Remove this field once we support new cartesian format. Doing so removes
                // the ambiguity between the actual distance (from calculations) and the distance defined
                // in the file.
                ui.label("Distance");
                changed = ui
                    .add(egui::DragValue::new(&mut self.2.distance).speed(0.1))
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
            changed = ui.add(egui::DragValue::new(&mut self.0)).changed() || changed;
            ui.end_row();

            ui.label("End");
            changed = ui.add(egui::DragValue::new(&mut self.1)).changed() || changed;
            ui.end_row();

            ui.label("Height");
            changed = ui
                .add(egui::DragValue::new(&mut self.2.texture_height).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Texture");
            changed = ui.text_edit_singleline(&mut *self.2.texture_name).changed() || changed;
            ui.end_row();

            ui.label("Alpha");
            changed = ui
                .add(egui::DragValue::new(&mut self.2.alpha).speed(0.01))
                .changed()
                || changed;
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

            ui.label("Name");
            changed = ui.text_edit_singleline(&mut self.instance_name).changed() || changed;
            ui.end_row();

            ui.label("Model");
            changed = ui.text_edit_singleline(&mut self.model_name).lost_focus() || changed;
            ui.end_row();
        });

        changed
    }
}

#[derive(Clone)]
struct EditableFloor {
    floor: Floor,
    vertices_str: String,
}

impl From<Floor> for EditableFloor {
    fn from(floor: Floor) -> Self {
        let vertices_str = floor
            .vertices
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",");
        Self {
            floor,
            vertices_str,
        }
    }
}

impl Editable for EditableFloor {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("floor").num_columns(2).show(ui, |ui| {
            ui.label("Texture");
            changed = ui
                .text_edit_singleline(&mut *self.floor.parameters.texture_name)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Texture Rotation");
            changed = ui
                .add(egui::DragValue::new(&mut self.floor.parameters.texture_rotation).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Texture Scale");
            changed = ui
                .add(egui::DragValue::new(&mut self.floor.parameters.texture_scale).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Vertices");
            if ui.text_edit_singleline(&mut self.vertices_str).lost_focus() {
                let mut parts = self.vertices_str.split(',');
                let vertices: Vec<usize> = parts
                    .by_ref()
                    .map_while(|s| s.parse::<usize>().ok())
                    .collect();
                if parts.next().is_none() {
                    self.floor.vertices = vertices;
                    changed = true;
                }
            }
            ui.end_row();
        });

        changed
    }
}

impl Editable for Door {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("door").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            changed = ui.text_edit_singleline(&mut *self.2.name).changed() || changed;
            ui.end_row();

            ui.label("X");
            changed = ui
                .add(egui::DragValue::new(&mut self.0).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Y");
            changed = ui
                .add(egui::DragValue::new(&mut self.1).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Type");
            egui::ComboBox::from_label("")
                .selected_text(DoorType::from(self.2.type_.as_str()).to_string())
                .show_ui(ui, |ui| {
                    for t in DOOR_TYPES {
                        changed = ui
                            .selectable_value(&mut *self.2.type_, t.to_value(), t.to_string())
                            .changed()
                            || changed;
                    }
                });
            ui.end_row();

            ui.label("Right Left Ratio");
            changed = ui
                .add(egui::DragValue::new(&mut self.2.right_left_ratio).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Motion Axis");
            changed = ui.text_edit_singleline(&mut *self.2.motion_axis).changed() || changed;
            ui.end_row();

            ui.label("Motion Degrees");
            changed = ui
                .add(egui::DragValue::new(&mut self.2.motion_degrees))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Motion Direction");
            changed = ui
                .add(egui::DragValue::new(&mut self.2.motion_direction))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Plugin");
            changed = ui.text_edit_singleline(&mut *self.2.plugin).changed() || changed;
            ui.end_row();
        });

        changed
    }
}

#[derive(Clone)]
struct EditableLift {
    name: String,
    lift: Lift,
    doors_json: String,
    valid_doors: bool,
    level_doors_json: String,
    valid_level_doors: bool,
}

impl EditableLift {
    pub fn from_lift(name: &str, lift: &Lift) -> serde_json::Result<Self> {
        Ok(Self {
            name: name.to_string(),
            lift: lift.clone(),
            doors_json: serde_json::to_string_pretty(&lift.doors)?,
            valid_doors: true,
            level_doors_json: serde_json::to_string_pretty(&lift.level_doors)?,
            valid_level_doors: true,
        })
    }
}

impl Editable for EditableLift {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("lift").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            changed = ui.text_edit_singleline(&mut self.name).changed() || changed;
            ui.end_row();

            ui.label("X");
            changed = ui
                .add(egui::DragValue::new(&mut self.lift.x).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Y");
            changed = ui
                .add(egui::DragValue::new(&mut self.lift.y).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Yaw");
            changed = ui
                .add(egui::DragValue::new(&mut self.lift.yaw).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Width");
            changed = ui
                .add(egui::DragValue::new(&mut self.lift.width).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Depth");
            changed = ui
                .add(egui::DragValue::new(&mut self.lift.depth).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Lowest Floor");
            changed = ui
                .text_edit_singleline(&mut self.lift.lowest_floor)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Highest Floor");
            changed = ui
                .text_edit_singleline(&mut self.lift.highest_floor)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Initial Floor");
            changed = ui
                .text_edit_singleline(&mut self.lift.initial_floor_name)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Reference Floor");
            changed = ui
                .text_edit_singleline(&mut self.lift.reference_floor_name)
                .changed()
                || changed;
            ui.end_row();

            ui.label("Plugins");
            changed = ui.checkbox(&mut self.lift.plugins, "").changed() || changed;
            ui.end_row();

            ui.label("Doors");
            changed = ui
                .add(TextEditJson::new(
                    &mut self.lift.doors,
                    &mut self.doors_json,
                    &mut self.valid_doors,
                ))
                .changed()
                || changed;
            ui.end_row();

            ui.label("Level Doors");
            changed = ui
                .add(TextEditJson::new(
                    &mut self.lift.level_doors,
                    &mut self.level_doors_json,
                    &mut self.valid_level_doors,
                ))
                .changed()
                || changed;
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
    Floor,
    Door,
    Lift,
}

enum EditorData {
    Lane(Lane),
    Vertex(Vertex),
    Measurement(Measurement),
    Wall(Wall),
    Model(Model),
    Floor(EditableFloor),
    Door(Door),
    Lift(EditableLift),
}

struct SelectedEditable(Entity, EditorData);

#[derive(Default)]
struct HasChanges(bool);

#[derive(SystemParam)]
struct EditorPanel<'w, 's> {
    q_lane: Query<'w, 's, &'static mut Lane>,
    q_vertex: Query<'w, 's, &'static mut Vertex>,
    q_measurement: Query<'w, 's, &'static mut Measurement>,
    q_wall: Query<'w, 's, &'static mut Wall>,
    q_model: Query<'w, 's, &'static mut Model>,
    q_floor: Query<'w, 's, &'static mut Floor>,
    q_door: Query<'w, 's, &'static mut Door>,
    q_lift: Query<'w, 's, &'static mut Lift>,
}

impl<'w, 's> EditorPanel<'w, 's> {
    fn draw(
        &mut self,
        ui: &mut egui::Ui,
        has_changes: &mut bool,
        mut selected: ResMut<Option<SelectedEditable>>,
    ) {
        fn commit_changes<E: Component + Clone>(
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

        let selected = match *selected {
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
            EditorData::Floor(_) => "Floor",
            EditorData::Door(_) => "Door",
            EditorData::Lift(_) => "Lift",
        };

        ui.heading(title);
        ui.separator();

        match &mut selected.1 {
            EditorData::Lane(lane) => {
                if lane.draw(ui) {
                    commit_changes(&mut self.q_lane, selected.0, lane);
                    *has_changes = true;
                }
            }
            EditorData::Vertex(vertex) => {
                if vertex.draw(ui) {
                    commit_changes(&mut self.q_vertex, selected.0, vertex);
                    *has_changes = true;
                }
            }
            EditorData::Measurement(measurement) => {
                if measurement.draw(ui) {
                    commit_changes(&mut self.q_measurement, selected.0, measurement);
                    *has_changes = true;
                }
            }
            EditorData::Wall(wall) => {
                if wall.draw(ui) {
                    commit_changes(&mut self.q_wall, selected.0, wall);
                    *has_changes = true;
                }
            }
            EditorData::Model(model) => {
                if model.draw(ui) {
                    commit_changes(&mut self.q_model, selected.0, model);
                    *has_changes = true;
                }
            }
            EditorData::Floor(floor) => {
                if floor.draw(ui) {
                    commit_changes(&mut self.q_floor, selected.0, &floor.floor);
                    *has_changes = true;
                }
            }
            EditorData::Door(door) => {
                if door.draw(ui) {
                    commit_changes(&mut self.q_door, selected.0, door);
                    *has_changes = true;
                }
            }
            EditorData::Lift(editable_lift) => {
                if editable_lift.draw(ui) {
                    commit_changes(&mut self.q_lift, selected.0, &editable_lift.lift);
                    *has_changes = true;
                }
            }
        };
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
    opened_map_file: Option<Res<OpenedMapFile>>,
    map: Res<BuildingMap>,
    mut save_map: EventWriter<SaveMap>,
    mut has_changes: ResMut<HasChanges>,
    mut spawner: Spawner,
    current_level: Option<Res<SiteMapCurrentLevel>>,
    mut selected: ResMut<Option<SelectedEditable>>,
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
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui
                        .add(egui::SelectableLabel::new(has_changes.0, "Save"))
                        .clicked()
                    {
                        if let Some(opened_file) = opened_map_file {
                            save_map.send(SaveMap(opened_file.0.clone()));
                        } else {
                            let path = rfd::FileDialog::new()
                                .set_file_name(&format!("{}.yaml", map.name))
                                .save_file();
                            if let Some(path) = path {
                                save_map.send(SaveMap(PathBuf::from(path)));
                            }
                        }
                        has_changes.0 = false;
                    }
                }
            });
        });
    });

    egui::SidePanel::right("editor_panel")
        .resizable(false)
        .default_width(250.)
        .max_width(250.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.group(|ui| {
                    if ui.button("Add Vertex").clicked() {
                        let new_vertex = Vertex::default();
                        let new_entity = spawner
                            .spawn_vertex(&current_level.as_ref().unwrap().0, new_vertex.clone())
                            .unwrap()
                            .id();
                        *selected =
                            Some(SelectedEditable(new_entity, EditorData::Vertex(new_vertex)));
                    }
                    if ui.button("Add Lane").clicked() {
                        let new_lane = Lane::default();
                        let new_entity = spawner
                            .spawn_in_level(&current_level.as_ref().unwrap().0, new_lane.clone())
                            .unwrap()
                            .id();
                        *selected = Some(SelectedEditable(new_entity, EditorData::Lane(new_lane)));
                    }
                    if ui.button("Add Measurement").clicked() {
                        let new_measurement = Measurement::default();
                        let new_entity = spawner
                            .spawn_in_level(
                                &current_level.as_ref().unwrap().0,
                                new_measurement.clone(),
                            )
                            .unwrap()
                            .id();
                        *selected = Some(SelectedEditable(
                            new_entity,
                            EditorData::Measurement(new_measurement),
                        ));
                    }
                    if ui.button("Add Wall").clicked() {
                        let new_wall = Wall::default();
                        let new_entity = spawner
                            .spawn_in_level(&current_level.as_ref().unwrap().0, new_wall.clone())
                            .unwrap()
                            .id();
                        *selected = Some(SelectedEditable(new_entity, EditorData::Wall(new_wall)));
                    }
                    if ui.button("Add Model").clicked() {
                        let new_model = Model::default();
                        let new_entity = spawner
                            .spawn_in_level(&current_level.as_ref().unwrap().0, new_model.clone())
                            .unwrap()
                            .id();
                        *selected =
                            Some(SelectedEditable(new_entity, EditorData::Model(new_model)));
                    }
                });
                ui.group(|ui| {
                    editor.draw(ui, &mut has_changes.0, selected);
                });
            });
        });
}

fn on_startup(
    mut commands: Commands,
    highlighting: Res<DefaultHighlighting<StandardMaterialHighlight>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mut hovered = mats.get_mut(&highlighting.hovered).unwrap();
    hovered.base_color = Color::rgb(0.35, 0.75, 0.35);
    let mut pressed = mats.get_mut(&highlighting.pressed).unwrap();
    pressed.base_color = Color::rgb(0.35, 0.35, 0.75);
    let mut selected = mats.get_mut(&highlighting.pressed).unwrap();
    selected.base_color = Color::rgb(0.35, 0.35, 0.75);

    commands
        .spawn()
        .insert(PickingBlocker)
        .insert(Interaction::default());
}

fn on_enter(
    mut commands: Commands,
    mut spawner: Spawner,
    building_map: Res<BuildingMap>,
    mut sitemap_state: ResMut<State<SiteMapState>>,
) {
    commands.insert_resource(HasChanges(false));
    spawner.spawn_map(&building_map);
    sitemap_state.set(SiteMapState::Enabled).unwrap();
}

fn on_exit(mut commands: Commands, mut sitemap_state: ResMut<State<SiteMapState>>) {
    commands.remove_resource::<BuildingMap>();
    commands.init_resource::<Option<SelectedEditable>>();
    sitemap_state.set(SiteMapState::Disabled).unwrap();
}

fn maintain_inspected_entities(
    editables: Query<(Entity, &Interaction, &EditableTag), Changed<Interaction>>,
    mut selected: ResMut<Option<SelectedEditable>>,
    q_lane: Query<&Lane>,
    q_vertex: Query<&Vertex>,
    q_measurement: Query<&Measurement>,
    q_wall: Query<&Wall>,
    q_model: Query<&Model>,
    q_floor: Query<&Floor>,
    q_door: Query<&Door>,
    q_lift: Query<&Lift>,
    q_name: Query<&basic_components::Name>,
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
        EditableTag::Floor => match q_floor.get(e) {
            Ok(floor) => Ok(SelectedEditable(
                e,
                EditorData::Floor(EditableFloor::from(floor.clone())),
            )),
            Err(err) => Err(err),
        },
        EditableTag::Door => match q_door.get(e) {
            Ok(door) => Ok(SelectedEditable(e, EditorData::Door(door.clone()))),
            Err(err) => Err(err),
        },
        EditableTag::Lift => match q_lift.get(e) {
            Ok(lift) => Ok(SelectedEditable(
                e,
                EditorData::Lift(EditableLift::from_lift(&q_name.get(e).unwrap().0, lift).unwrap()),
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
    lanes: Query<Entity, With<Lane>>,
    vertices: Query<Entity, With<Vertex>>,
    measurements: Query<Entity, With<Measurement>>,
    walls: Query<Entity, With<Wall>>,
    models: Query<Entity, With<Model>>,
    floors: Query<Entity, With<Floor>>,
    doors: Query<Entity, With<Door>>,
    lifts: Query<Entity, With<Lift>>,
    meshes: Query<Entity, Changed<Handle<Mesh>>>,
    parent: Query<&Parent>,
    selected: Res<Option<SelectedEditable>>,
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Query<&mut Interaction, With<PickingBlocker>>,
) {
    // bevy_mod_picking only works on entities with meshes, the models are spawned with
    // child scenes so making the model entity pickable will not work. We need to check
    // all meshes added and go up the hierarchy to find if the mesh is part of a model.
    //
    // As of bevy_mod_picking 0.7, highlighting no longer works if an entity is made pickable
    // before it has a mesh (likely for performance reasons), so we loop through added meshes
    // instead of added lanes/vertices etc.
    for mesh_entity in meshes.iter() {
        // go up the hierarchy tree until the root, trying to find if a mesh is part of a model.
        let mut e = Some(mesh_entity);
        let mut tag: Option<EditableTag> = None;
        while let Some(cur) = e {
            if lanes.contains(cur) {
                tag = Some(EditableTag::Lane);
            }
            if vertices.contains(cur) {
                tag = Some(EditableTag::Vertex);
            }
            if walls.contains(cur) {
                tag = Some(EditableTag::Wall);
            }
            if measurements.contains(cur) {
                tag = Some(EditableTag::Measurement);
            }
            if floors.contains(cur) {
                tag = Some(EditableTag::Floor);
            }
            if doors.contains(cur) {
                tag = Some(EditableTag::Door);
            }
            if lifts.contains(cur) {
                tag = Some(EditableTag::Lift);
            }

            // check if this entity is a model, if so, make it pickable.
            if let Ok(model_entity) = models.get(cur) {
                tag = Some(EditableTag::Model(model_entity));
            }

            if let Some(tag) = tag {
                // Some objects may respawn their mesh they are changed, causing `bevy_mod_picking`
                // to forget that the mesh is selected. Workaround that by forcing the selected state.
                //
                // FIXME: This still creates a one-frame lag where the mesh is not highlighted.
                // This is because we only know of meshes added in the next frame since commands
                // are ran at the end of a stage. The bevy stageless RFC may fix this, but for now
                // we need to move this into a custom stage to fix this.
                let mut selection = Selection::default();
                if let Some(selected) = &*selected {
                    if selected.0 == cur {
                        selection.set_selected(true);
                    }
                }

                commands
                    .entity(mesh_entity)
                    .insert_bundle(PickableBundle {
                        selection,
                        ..default()
                    })
                    .insert(tag);
                break;
            }

            e = match parent.get(cur) {
                Ok(parent) => Some(parent.0),
                Err(_) => None,
            };
        }
    }

    // Stops picking when egui is in focus.
    // This creates a dummy PickingBlocker and make it "Clicked" whenever egui is in focus.
    //
    // Normally bevy_mod_picking automatically stops when
    // a bevy ui node is in focus, but bevy_egui does not use bevy ui node.
    let egui_ctx = egui_context.ctx_mut();
    let enable = !egui_ctx.wants_pointer_input()
        && !egui_ctx.wants_keyboard_input()
        && !egui_ctx.is_pointer_over_area();

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
            .init_resource::<HasChanges>()
            .add_startup_system(on_startup)
            .add_system_set(SystemSet::on_enter(AppState::TrafficEditor).with_system(on_enter))
            .add_system_set(SystemSet::on_exit(AppState::TrafficEditor).with_system(on_exit))
            .add_system_set(
                SystemSet::on_update(AppState::TrafficEditor)
                    .before(SiteMapLabel)
                    .with_system(egui_ui)
                    .with_system(update_picking_cam)
                    // must be after egui_ui so that the picking blocker knows about all the ui elements
                    .with_system(enable_picking.after(egui_ui))
                    .with_system(maintain_inspected_entities),
            );
    }
}
