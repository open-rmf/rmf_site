use std::path::PathBuf;

use crate::basic_components;
use crate::building_map::BuildingMap;
use crate::camera_controls::{CameraControls, ProjectionMode};
use crate::door::{Door, DoorType, DOOR_TYPES};
use crate::floor::Floor;
use crate::interaction::{Hovering, InteractionPlugin, Selected};
use crate::lane::Lane;
use crate::lift::Lift;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::save_load::SaveMap;
use crate::site_map::{SiteMapCurrentLevel, SiteMapLabel, SiteMapState};
use crate::spawner::{Spawner, VerticesManagers};
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::widgets::TextEditJson;
use crate::{AppState, OpenedMapFile};
use bevy::ecs::system::SystemParam;
use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_egui::{egui, EguiContext};
use bevy_mod_picking::{
    mesh_focus, pause_for_picking_blockers, PausedForBlockers, PickableBundle, PickableMesh,
    PickingBlocker, PickingCamera, PickingCameraBundle, PickingPlugin, PickingPluginsState,
    PickingSystem,
};

#[derive(Debug)]
pub struct ElementDeleted(pub Entity);

trait Editable {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool;
}

impl Editable for Vertex {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("vertex").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            changed = ui.text_edit_singleline(&mut self.3).changed() || changed;
            ui.end_row();

            ui.label("x");
            changed = ui
                .add(egui::DragValue::new(&mut self.0).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("y");
            changed = ui
                .add(egui::DragValue::new(&mut self.1).speed(0.1))
                .changed()
                || changed;
            ui.end_row();

            ui.label("z");
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
            ui.label("Start Vertex");
            ui.label(format!("{}", self.0));
            ui.end_row();

            ui.label("End Vertex");
            ui.label(format!("{}", self.1));
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
                ui.label("Start Vertex");
                ui.label(format!("{}", self.0));
                ui.end_row();

                ui.label("End Vertex");
                ui.label(format!("{}", self.1));
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
            ui.label("Start Vertex");
            ui.label(format!("{}", self.0));
            ui.end_row();

            ui.label("End Vertex");
            ui.label(format!("{}", self.1));
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

            ui.label("Start Vertex");
            ui.label(format!("{}", self.0));
            ui.end_row();

            ui.label("End Vertex");
            ui.label(format!("{}", self.1));
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

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableTag {
    Lane(Entity),
    Vertex(Entity),
    Measurement(Entity),
    Wall(Entity),
    Model(Entity),
    Floor(Entity),
    Door(Entity),
    Lift(Entity),
    /// Apply this tag to entities which may be a child of an editable item
    /// but should be ignored by the picker
    Ignore,
}

impl EditableTag {
    fn unwrap_entity(&self) -> Entity {
        self.entity().unwrap()
    }

    pub fn entity(&self) -> Option<Entity> {
        match self {
            Self::Lane(e) => Some(*e),
            Self::Vertex(e) => Some(*e),
            Self::Measurement(e) => Some(*e),
            Self::Wall(e) => Some(*e),
            Self::Model(e) => Some(*e),
            Self::Floor(e) => Some(*e),
            Self::Door(e) => Some(*e),
            Self::Lift(e) => Some(*e),
            Self::Ignore => None,
        }
    }

    pub fn ignore(&self) -> bool {
        match self {
            Self::Ignore => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
struct SelectedEditable(pub EditableTag, pub EditorData);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct HoveredEditable(pub Entity);

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
        vm: &VerticesManagers,
        level: &SiteMapCurrentLevel,
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

        let selected = match selected.as_mut() {
            Some(selected) => selected,
            None => {
                ui.add_sized(ui.available_size(), egui::Label::new("No object selected"));
                return;
            }
        };

        // INVARIANT: We should never have Some(selected) with an EditableTag::Ignore
        // value in it.
        let e = selected.0.unwrap_entity();
        let title = match &selected.1 {
            EditorData::Vertex(_) => {
                if let Some(vm) = vm.0.get(&level.0) {
                    if let Some(v_id) = vm.entity_to_id(e) {
                        format!("Vertex #{v_id}")
                    } else {
                        format!("Vertex <Unknown Entity: {:?}>", e)
                    }
                } else {
                    format!("Vertex <Unknown level: {}>", level.0)
                }
            }
            EditorData::Lane(_) => "Lane".to_string(),
            EditorData::Measurement(_) => "Measurement".to_string(),
            EditorData::Wall(_) => "Wall".to_string(),
            EditorData::Model(_) => "Model".to_string(),
            EditorData::Floor(_) => "Floor".to_string(),
            EditorData::Door(_) => "Door".to_string(),
            EditorData::Lift(_) => "Lift".to_string(),
        };

        ui.heading(title);
        ui.separator();

        match &mut selected.1 {
            EditorData::Lane(lane) => {
                if lane.draw(ui) {
                    commit_changes(&mut self.q_lane, e, lane);
                    *has_changes = true;
                }
            }
            EditorData::Vertex(vertex) => {
                if vertex.draw(ui) {
                    commit_changes(&mut self.q_vertex, e, vertex);
                    *has_changes = true;
                }
            }
            EditorData::Measurement(measurement) => {
                if measurement.draw(ui) {
                    commit_changes(&mut self.q_measurement, e, measurement);
                    *has_changes = true;
                }
            }
            EditorData::Wall(wall) => {
                if wall.draw(ui) {
                    commit_changes(&mut self.q_wall, e, wall);
                    *has_changes = true;
                }
            }
            EditorData::Model(model) => {
                if model.draw(ui) {
                    commit_changes(&mut self.q_model, e, model);
                    *has_changes = true;
                }
            }
            EditorData::Floor(floor) => {
                if floor.draw(ui) {
                    commit_changes(&mut self.q_floor, e, &floor.floor);
                    *has_changes = true;
                }
            }
            EditorData::Door(door) => {
                if door.draw(ui) {
                    commit_changes(&mut self.q_door, e, door);
                    *has_changes = true;
                }
            }
            EditorData::Lift(editable_lift) => {
                if editable_lift.draw(ui) {
                    commit_changes(&mut self.q_lift, e, &editable_lift.lift);
                    *has_changes = true;
                }
            }
        };
    }
}

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut q_camera_controls: Query<&mut CameraControls>,
    mut cameras: Query<(&mut Camera, &mut Visibility)>,
    mut app_state: ResMut<State<AppState>>,
    mut editor: EditorPanel,
    opened_map_file: Option<Res<OpenedMapFile>>,
    map: Res<BuildingMap>,
    mut save_map: EventWriter<SaveMap>,
    mut has_changes: ResMut<HasChanges>,
    mut spawner: Spawner,
    current_level: Res<Option<SiteMapCurrentLevel>>,
    selected: ResMut<Option<SelectedEditable>>,
    mut select: EventWriter<Option<SelectedEditable>>,
) {
    let mut controls = q_camera_controls.single_mut();
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
                        controls.mode() == ProjectionMode::Orthographic,
                        "2D",
                    ))
                    .clicked()
                {
                    controls.use_orthographic(true, &mut cameras);
                }
                if ui
                    .add(egui::SelectableLabel::new(
                        controls.mode() == ProjectionMode::Perspective,
                        "3D",
                    ))
                    .clicked()
                {
                    controls.use_perspective(true, &mut cameras);
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
                if let Some(current_level) = current_level.as_ref() {
                    ui.group(|ui| {
                        if ui.button("Add Vertex").clicked() {
                            let new_vertex = Vertex::default();
                            let new_entity = spawner
                                .spawn_vertex(&current_level.0, new_vertex.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Vertex(new_entity),
                                EditorData::Vertex(new_vertex),
                            )));
                        }
                        if ui.button("Add Lane").clicked() {
                            let new_lane = Lane::default();
                            let new_entity = spawner
                                .spawn_in_level(&current_level.0, new_lane.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Lane(new_entity),
                                EditorData::Lane(new_lane),
                            )));
                        }
                        if ui.button("Add Measurement").clicked() {
                            let new_measurement = Measurement::default();
                            let new_entity = spawner
                                .spawn_in_level(&current_level.0, new_measurement.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Measurement(new_entity),
                                EditorData::Measurement(new_measurement),
                            )));
                        }
                        if ui.button("Add Wall").clicked() {
                            let new_wall = Wall::default();
                            let new_entity = spawner
                                .spawn_in_level(&current_level.0, new_wall.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Wall(new_entity),
                                EditorData::Wall(new_wall),
                            )));
                        }
                        if ui.button("Add Model").clicked() {
                            let new_model = Model::default();
                            let new_entity = spawner
                                .spawn_in_level(&current_level.0, new_model.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Model(new_entity),
                                EditorData::Model(new_model),
                            )));
                        }
                        if ui.button("Add Door").clicked() {
                            let new_door = Door::default();
                            let new_entity = spawner
                                .spawn_in_level(&current_level.0, new_door.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Door(new_entity),
                                EditorData::Door(new_door),
                            )));
                        }
                        if ui.button("Add Lift").clicked() {
                            let cur_level = &current_level.0;
                            let new_lift = Lift {
                                initial_floor_name: cur_level.clone(),
                                ..default()
                            };
                            let new_entity = spawner
                                .spawn_in_level(&cur_level, new_lift.clone())
                                .unwrap()
                                .id();
                            select.send(Some(SelectedEditable(
                                EditableTag::Lift(new_entity),
                                EditorData::Lift(
                                    EditableLift::from_lift("new_lift", &new_lift).unwrap(),
                                ),
                            )));
                        }
                    });
                    ui.group(|ui| {
                        editor.draw(
                            ui,
                            spawner.vertex_mgrs.as_ref(),
                            current_level,
                            &mut has_changes.0,
                            selected,
                        );
                    });
                }
            });
        });
}

fn on_startup(mut commands: Commands) {
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

fn check_and_delete_vertex(
    entity: Entity,
    lanes: Query<&Lane>,
    walls: Query<&Wall>,
    measurements: Query<&Measurement>,
    mut vertices_mgrs: ResMut<VerticesManagers>,
) -> bool {
    // Find its vertex id from the vertices_mgrs
    for mgr in vertices_mgrs.0.iter_mut() {
        match mgr.1.entity_to_id(entity) {
            Some(id) => {
                // Now go through all edges
                for lane in lanes.iter() {
                    if lane.0 == id || lane.1 == id {
                        println!("Cannot delete vertex, used in a lane");
                        return false;
                    }
                }
                for wall in walls.iter() {
                    if wall.0 == id || wall.1 == id {
                        println!("Cannot delete vertex, used in a wall");
                        return false;
                    }
                }
                for meas in measurements.iter() {
                    if meas.0 == id || meas.1 == id {
                        println!("Cannot delete vertex, used in a measurement");
                        return false;
                    }
                }
                // Bookkeeping with the vertices manager
                mgr.1.remove(id);
                return true;
            }
            None => {}
        }
    }
    // This should never happen
    println!("Vertex not found in manager, please report this bug");
    return false;
}

fn handle_keyboard_events(
    mut commands: Commands,
    lanes: Query<&Lane>,
    walls: Query<&Wall>,
    measurements: Query<&Measurement>,
    vertices: Query<Entity, With<Vertex>>,
    vertices_mgrs: ResMut<VerticesManagers>,
    keys: Res<Input<KeyCode>>,
    mut has_changes: ResMut<HasChanges>,
    mut delete_events: EventWriter<ElementDeleted>,
    selected: Res<Option<SelectedEditable>>,
    mut select: EventWriter<Option<SelectedEditable>>,
) {
    // Delete model if selected and delete was pressed
    if keys.just_pressed(KeyCode::Delete) {
        // We need to clear selection regardless, hence take the option
        match selected.as_ref() {
            Some(sel) => {
                let entity = sel.0.unwrap_entity();
                let mut safe_to_delete = true;
                // We can't delete vertices that are still in use
                if vertices.get(entity).is_ok() {
                    safe_to_delete =
                        check_and_delete_vertex(entity, lanes, walls, measurements, vertices_mgrs);
                }
                if safe_to_delete {
                    delete_events.send(ElementDeleted(entity));
                    commands.entity(entity).despawn_recursive();
                    select.send(None);
                    has_changes.0 = true;
                }
            }
            None => println!("Nothing selected"),
        }
    } else if keys.just_pressed(KeyCode::Escape) {
        // TODO Picking highlighting is not cleared, fix
        select.send(None);
    }
}

impl<'w, 's> EditableQuery<'w, 's> {
    fn get_selected_data(&self, tag: &EditableTag) -> Option<SelectedEditable> {
        let result = match tag {
            // Clone and draw an inspectable so as to avoid change detection in bevy.
            // This also allows us to commit changes only when needed, e.g. commit only
            // when the user press "enter" when editing a text field.
            //
            // Bevy change detection works by implementing the dereference operator to mark something
            // as changed, this cause the change detection to trigger even if there are no writes to
            // it. Egui on the other hand requires data to be mutable, so passing a component directly
            // to egui will cause change detection to trigger every frame.
            EditableTag::Lane(entity) => self
                .q_lane
                .get(*entity)
                .map(|lane| Some(SelectedEditable(*tag, EditorData::Lane(lane.clone())))),
            EditableTag::Vertex(entity) => self
                .q_vertex
                .get(*entity)
                .map(|vertex| Some(SelectedEditable(*tag, EditorData::Vertex(vertex.clone())))),
            EditableTag::Measurement(entity) => self
                .q_measurement
                .get(*entity)
                .map(|m| Some(SelectedEditable(*tag, EditorData::Measurement(m.clone())))),
            EditableTag::Wall(entity) => self
                .q_wall
                .get(*entity)
                .map(|w| Some(SelectedEditable(*tag, EditorData::Wall(w.clone())))),
            EditableTag::Model(entity) => self
                .q_model
                .get(*entity)
                .map(|m| Some(SelectedEditable(*tag, EditorData::Model(m.clone())))),
            EditableTag::Floor(entity) => self
                .q_floor
                .get(*entity)
                .map(|f| Some(SelectedEditable(*tag, EditorData::Floor(f.clone().into())))),
            EditableTag::Door(entity) => self
                .q_door
                .get(*entity)
                .map(|d| Some(SelectedEditable(*tag, EditorData::Door(d.clone())))),
            EditableTag::Lift(entity) => self.q_lift.get(*entity).map(|l| {
                Some(SelectedEditable(
                    *tag,
                    EditorData::Lift(
                        EditableLift::from_lift(&self.q_name.get(*entity).unwrap().0, l).unwrap(),
                    ),
                ))
            }),
            EditableTag::Ignore => Ok(None),
        };

        match result {
            Ok(selected) => selected,
            Err(err) => {
                println!("{err}");
                None
            }
        }
    }
}

fn handle_interactions(
    interactions: Query<(&Interaction, &EditableTag), Changed<Interaction>>,
    paused: Option<Res<PausedForBlockers>>,
    editables: EditableQuery,
    mut select: EventWriter<Option<SelectedEditable>>,
    mut hover: EventWriter<Option<HoveredEditable>>,
) {
    if let Some(paused) = paused {
        if paused.is_paused() {
            return;
        }
    }

    let clicked = interactions.iter().find(|(i, _)| match i {
        Interaction::Clicked => true,
        _ => false,
    });
    if let Some((_, tag)) = clicked {
        select.send(editables.get_selected_data(tag));
    }

    let new_hovered = interactions
        .iter()
        .find(|(i, _)| match i {
            Interaction::Hovered => true,
            _ => false,
        })
        .filter(|(_, tag)| !tag.ignore())
        .map(|(_, tag)| HoveredEditable(tag.unwrap_entity().clone()));
    if let Some(current) = new_hovered {
        hover.send(Some(current));
    }
}

fn maintain_inspected_entities(
    mut hovering: Query<&mut Hovering>,
    mut selection: Query<&mut Selected>,
    mut selected: ResMut<Option<SelectedEditable>>,
    mut hovered: ResMut<Option<HoveredEditable>>,
    mut select: EventReader<Option<SelectedEditable>>,
    mut hover: EventReader<Option<HoveredEditable>>,
) {
    let previous_selected = selected.as_ref().as_ref().map(|s| s.0.clone());
    if let Some(newly_selected) = select.iter().last() {
        if newly_selected.as_ref().map(|s| s.0) != previous_selected {
            *selected = newly_selected.clone();
            let selected_tag = selected.as_ref().as_ref().map(|s| s.0.clone());
            if previous_selected != selected_tag {
                if let Some(previous) = previous_selected {
                    if let Some(mut selected) = selection.get_mut(previous.unwrap_entity()).ok() {
                        selected.is_selected = false;
                    }
                }

                if let Some(current) = selected_tag {
                    if let Some(mut selected) = selection.get_mut(current.unwrap_entity()).ok() {
                        selected.is_selected = true;
                    }
                }
            }
        }
    }

    let previous_hovered = *hovered;
    if let Some(current) = hover.iter().last() {
        if previous_hovered != *current {
            *hovered = *current;
            if let Some(previous) = previous_hovered {
                if let Some(mut hovered) = hovering.get_mut(previous.0).ok() {
                    hovered.is_hovering = false;
                }
            }

            if let Some(current) = current {
                if let Some(mut hovered) = hovering.get_mut(current.0).ok() {
                    hovered.is_hovering = true;
                }
            }
        }
    }
}

fn update_picking_cam(
    mut commands: Commands,
    camera_controls: Query<(&CameraControls, ChangeTrackers<CameraControls>)>,
    picking_cams: Query<Entity, With<PickingCamera>>,
) {
    let (controls, changed) = camera_controls.single();
    if changed.is_changed() {
        let active_camera = controls.active_camera();
        if picking_cams
            .get_single()
            .ok()
            .filter(|current| *current == active_camera)
            .is_none()
        {
            for cam in picking_cams.iter() {
                commands.entity(cam).remove_bundle::<PickingCameraBundle>();
            }

            commands
                .entity(controls.active_camera())
                .insert_bundle(PickingCameraBundle::default());
        }
    }
}

fn add_editable_tags(
    mut commands: Commands,
    lanes: Query<Entity, Added<Lane>>,
    vertices: Query<Entity, Added<Vertex>>,
    measurements: Query<Entity, Added<Measurement>>,
    walls: Query<Entity, Added<Wall>>,
    models: Query<Entity, Added<Model>>,
    floors: Query<Entity, Added<Floor>>,
    doors: Query<Entity, Added<Door>>,
    lifts: Query<Entity, Added<Lift>>,
    meshes: Query<Entity, With<Handle<Mesh>>>,
) {
    // TODO(MXG): Consider a macro to get rid of this boilerplate
    for e in &lanes {
        commands.entity(e).insert(EditableTag::Lane(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &vertices {
        commands.entity(e).insert(EditableTag::Vertex(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &measurements {
        commands.entity(e).insert(EditableTag::Measurement(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &walls {
        commands.entity(e).insert(EditableTag::Wall(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &models {
        commands.entity(e).insert(EditableTag::Model(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &floors {
        commands.entity(e).insert(EditableTag::Floor(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &doors {
        commands.entity(e).insert(EditableTag::Door(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }

    for e in &lifts {
        commands.entity(e).insert(EditableTag::Lift(e));
        if meshes.contains(e) {
            commands.entity(e).insert_bundle(PickableBundle::default());
        }
    }
}

fn enable_picking_editables(
    mut commands: Commands,
    editables: Query<(Entity, &EditableTag), (Added<Handle<Mesh>>, Without<PickableMesh>)>,
) {
    // If any editable item gets a new mesh, make it pickable
    for (entity, tag) in &editables {
        if *tag != EditableTag::Ignore {
            commands
                .entity(entity)
                .insert_bundle(PickableBundle::default());
        }
    }
}

fn propagate_editable_tags(
    mut commands: Commands,
    // All entities with an editable tag whose children have changed
    needs_to_propagate_tag: Query<(&Children, &EditableTag), Changed<Children>>,
    // All entities that have a parent but do not currently have an editable tag
    might_need_to_receive_tag: Query<
        (Entity, Option<&Children>),
        (With<Parent>, Without<EditableTag>),
    >,
    meshes: Query<Entity, With<Handle<Mesh>>>,
) {
    for parent in &needs_to_propagate_tag {
        recursive_propagate_editable_tags(
            &mut commands,
            parent,
            &might_need_to_receive_tag,
            &meshes,
        );
    }
}

fn recursive_propagate_editable_tags(
    commands: &mut Commands,
    (children, tag): (&Children, &EditableTag),
    might_need_to_receive_tag: &Query<
        (Entity, Option<&Children>),
        (With<Parent>, Without<EditableTag>),
    >,
    meshes: &Query<Entity, With<Handle<Mesh>>>,
) {
    if *tag == EditableTag::Ignore {
        return;
    }

    for child in children {
        if let Some((child, grandchildren)) = might_need_to_receive_tag.get(*child).ok() {
            commands.entity(child).insert(*tag);
            if meshes.contains(child) {
                commands
                    .entity(child)
                    .insert_bundle(PickableBundle::default());
            }

            if let Some(grandchildren) = grandchildren {
                recursive_propagate_editable_tags(
                    commands,
                    (grandchildren, tag),
                    might_need_to_receive_tag,
                    meshes,
                );
            }
        }
    }
}

fn egui_picking_blocker(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Query<&mut Interaction, With<PickingBlocker>>,
) {
    // Stops picking when egui is in focus.
    // This creates a dummy PickingBlocker and make it "Clicked" whenever egui is in focus.
    //
    // Normally bevy_mod_picking automatically stops when
    // a bevy ui node is in focus, but bevy_egui does not use bevy ui node.
    let egui_ctx = egui_context.ctx_mut();
    let enable = !egui_ctx.wants_pointer_input()
        && !egui_ctx.wants_keyboard_input()
        && !egui_ctx.is_pointer_over_area();

    if enable {
        // Check if we need to actually change the state of the component before
        // we do a mutable borrow. Otherwise it will needlessly trigger systems
        // that are tracking changes for the component.
        if *picking_blocker.single() != Interaction::None {
            *picking_blocker.single_mut() = Interaction::None;
        }
    } else {
        if *picking_blocker.single() != Interaction::Clicked {
            *picking_blocker.single_mut() = Interaction::Clicked;
        }
    }
}

#[derive(SystemParam)]
pub struct EditableQuery<'w, 's> {
    q_lane: Query<'w, 's, &'static Lane>,
    q_vertex: Query<'w, 's, &'static Vertex>,
    q_measurement: Query<'w, 's, &'static Measurement>,
    q_wall: Query<'w, 's, &'static Wall>,
    q_model: Query<'w, 's, &'static Model>,
    q_floor: Query<'w, 's, &'static Floor>,
    q_door: Query<'w, 's, &'static Door>,
    q_lift: Query<'w, 's, &'static Lift>,
    q_name: Query<'w, 's, &'static basic_components::Name>,
}

#[derive(Default)]
pub struct TrafficEditorPlugin;

impl Plugin for TrafficEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InteractionPlugin::new(AppState::TrafficEditor))
            .init_resource::<Option<SelectedEditable>>()
            .init_resource::<Option<HoveredEditable>>()
            .init_resource::<HasChanges>()
            .add_event::<ElementDeleted>()
            .add_event::<Option<SelectedEditable>>()
            .add_event::<Option<HoveredEditable>>()
            .add_startup_system(on_startup)
            .add_system_set(SystemSet::on_enter(AppState::TrafficEditor).with_system(on_enter))
            .add_system_set(SystemSet::on_exit(AppState::TrafficEditor).with_system(on_exit))
            .add_system_set(
                SystemSet::on_update(AppState::TrafficEditor)
                    .after(SiteMapLabel)
                    .with_system(egui_ui)
                    .with_system(egui_picking_blocker.after(egui_ui))
                    .with_system(update_picking_cam)
                    .with_system(handle_keyboard_events)
                    // must be after egui_ui so that the picking blocker knows about all the ui elements
                    .with_system(add_editable_tags.after(egui_ui))
                    .with_system(propagate_editable_tags.after(add_editable_tags))
                    .with_system(enable_picking_editables),
            )
            .add_plugin(PickingPlugin)
            .init_resource::<PausedForBlockers>()
            .add_system_set_to_stage(
                CoreStage::First,
                SystemSet::new()
                    .with_run_criteria(
                        |state: (Res<PickingPluginsState>, Res<Option<SiteMapCurrentLevel>>)| {
                            if state.1.is_none() {
                                return ShouldRun::No;
                            }

                            if state.0.enable_interacting {
                                ShouldRun::Yes
                            } else {
                                ShouldRun::No
                            }
                        },
                    )
                    .with_system(
                        pause_for_picking_blockers
                            .label(PickingSystem::PauseForBlockers)
                            .after(PickingSystem::UpdateIntersections),
                    )
                    .with_system(
                        mesh_focus
                            .label(PickingSystem::Focus)
                            .after(PickingSystem::PauseForBlockers),
                    )
                    .with_system(
                        handle_interactions
                            .label(PickingSystem::Selection)
                            .after(PickingSystem::Focus),
                    )
                    .with_system(maintain_inspected_entities.after(PickingSystem::Selection)),
            );
    }
}
