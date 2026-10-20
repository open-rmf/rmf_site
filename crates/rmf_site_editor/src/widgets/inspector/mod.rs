/*
 * Copyright (C) 2022 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

pub mod inspect_associated_graphs;
pub use inspect_associated_graphs::*;

pub mod inspect_anchor;
pub use inspect_anchor::*;

pub mod inspect_angle;
pub use inspect_angle::*;

pub mod inspect_asset_source;
pub use inspect_asset_source::*;

pub mod inspect_door;
pub use inspect_door::*;

pub mod inspect_drawing;
pub use inspect_drawing::*;

pub mod inspect_edge;
pub use inspect_edge::*;

pub mod inspect_fiducial;
pub use inspect_fiducial::*;

pub mod inspect_geography;
pub use inspect_geography::*;

pub mod inspect_group;
pub use inspect_group::*;

pub mod inspect_height;
pub use inspect_height::*;

pub mod inspect_is_static;
pub use inspect_is_static::*;

pub mod inspect_option_string;
pub use inspect_option_string::*;

pub mod inspect_layer;
pub use inspect_layer::*;

pub mod inspect_level;
pub use inspect_level::*;

pub mod inspect_lift;
pub use inspect_lift::*;

pub mod inspect_light;
pub use inspect_light::*;

pub mod inspect_location;
pub use inspect_location::*;

pub mod inspect_point;
pub use inspect_point::*;

pub mod inspect_primitive_shape;
pub use inspect_primitive_shape::*;

pub mod inspect_measurement;
pub use inspect_measurement::*;

pub mod inspect_model_description;
pub use inspect_model_description::*;

pub mod inspect_motion;
pub use inspect_motion::*;

pub mod inspect_multi_selection;
pub use inspect_multi_selection::*;

pub mod inspect_mutex;
pub use inspect_mutex::*;

pub mod inspect_name;
pub use inspect_name::*;

pub mod inspect_option_f32;
pub use inspect_option_f32::*;

pub mod inspect_physical_camera_properties;
pub use inspect_physical_camera_properties::*;

pub mod inspect_pose;
pub use inspect_pose::*;

pub mod inspect_preview;
pub use inspect_preview::*;

pub mod inspect_scale;
pub use inspect_scale::*;

pub mod inspect_side;
pub use inspect_side::*;

pub mod inspect_texture;
pub use inspect_texture::*;

pub mod inspect_value;
pub use inspect_value::*;
use rmf_site_picking::{Select, Selected, Selection};

use crate::{
    site::{Category, SiteID},
    CurrentWorkspace,
};
use bevy::{
    ecs::{
        hierarchy::ChildOf,
        relationship::AncestorIter,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, Color32, Key, RichText, TextEdit, Ui};
use rmf_site_egui::*;
use rmf_site_format::*;
use smallvec::SmallVec;

/// Use this plugin to add a single inspection tile into the [`MainInspector`]
/// widget.
///
/// ```no_run
/// use bevy::prelude::{App, Query, Entity, Res};
/// use rmf_site_editor::{SiteEditor, site::NameInSite, widgets::prelude::*};
/// use rmf_site_egui::*;
///
/// #[derive(SystemParam)]
/// pub struct HelloSelection<'w, 's> {
///     names: Query<'w, 's, &'static NameInSite>,
/// }
///
/// impl<'w, 's> WidgetSystem<Inspect> for HelloSelection<'w, 's> {
///     fn show(
///         Inspect { selection, .. }: Inspect,
///         ui: &mut Ui,
///         state: &mut SystemState<Self>,
///         world: &mut World,
///     ) {
///         let mut params = state.get_mut(world);
///         let name = params.names.get(selection)
///             .map(|name| name.as_str())
///             .unwrap_or("<unknown>");
///         ui.add_space(20.0);
///         ui.heading(format!("Hello, {name}!"));
///         ui.add_space(20.0);
///     }
/// }
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((
///         SiteEditor::default(),
///         InspectionPlugin::<HelloSelection>::new(),
///     ));
///
///     app.run();
/// }
/// ```
pub struct InspectionPlugin<W>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
{
    _ignore: std::marker::PhantomData<W>,
}

/// Use this to create a standard inspector plugin that covers the common use
/// cases of the site editor.
#[derive(Default)]
pub struct StandardInspectorPlugin {}

impl Plugin for StandardInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MinimalInspectorPlugin::default())
            .add_plugins((
                InspectionPlugin::<InspectName>::new(),
                InspectionPlugin::<InspectSelectedModelDescription>::new(),
                InspectionPlugin::<InspectAnchor>::new(),
                InspectionPlugin::<InspectAnchorDependents>::new(),
                InspectionPlugin::<InspectEdge>::new(),
                InspectionPlugin::<InspectPoint>::new(),
                InspectionPlugin::<InspectGeography>::new(),
                InspectFiducialPlugin::default(),
                InspectionPlugin::<InspectLayer>::new(),
                InspectionPlugin::<InspectDrawing>::new(),
                InspectionPlugin::<InspectAssetSource>::new(),
                InspectionPlugin::<InspectAssociatedGraphs>::new(),
                InspectionPlugin::<InspectLocation>::new(),
                InspectTexturePlugin::default(),
                InspectionPlugin::<InspectMotion>::new(),
                // Reached the tuple limit
            ))
            .add_plugins((
                InspectMutexPlugin::default(),
                InspectionPlugin::<InspectPose>::new(),
                InspectionPlugin::<InspectScale>::new(),
                InspectionPlugin::<InspectLight>::new(),
                InspectionPlugin::<InspectDoor>::new(),
                InspectionPlugin::<InspectHeight>::new(),
                InspectionPlugin::<InspectPrimitiveShape>::new(),
                InspectionPlugin::<InspectMeasurement>::new(),
                InspectionPlugin::<InspectPhysicalCameraProperties>::new(),
                InspectionPlugin::<InspectPreview>::new(),
                InspectionPlugin::<InspectGroup>::new(),
                InspectionPlugin::<InspectLevel>::new(),
                InspectModelDescriptionPlugin::default(),
                InspectLiftPlugin::default(),
            ))
            .add_plugins(
                (
                    // Required model properties
                    InspectModelPropertyPlugin::<InspectModelScale, Scale>::new(
                        "Scale".to_string(),
                    ),
                    InspectModelPropertyPlugin::<InspectModelAssetSource, AssetSource>::new(
                        "Asset Source".to_string(),
                    ),
                    InspectRobotPropertiesPlugin::default(),
                    InspectRobotPropertyPlugin::<InspectMobility, Mobility>::new(),
                    InspectRobotPropertyPlugin::<InspectCollision, Collision>::new(),
                    InspectRobotPropertyPlugin::<InspectPowerSource, PowerSource>::new(),
                    InspectRobotPropertyPlugin::<InspectPowerDissipation, PowerDissipation>::new(),
                    InspectRobotPropertyKindPlugin::<
                        InspectDifferentialDrive,
                        DifferentialDrive,
                        Mobility,
                    >::new(),
                    InspectRobotPropertyKindPlugin::<
                        InspectCircleCollision,
                        CircleCollision,
                        Collision,
                    >::new(),
                    InspectRobotPropertyKindPlugin::<InspectBattery, Battery, PowerSource>::new(),
                    InspectRobotPropertyKindPlugin::<
                        InspectAmbientSystem,
                        AmbientSystem,
                        PowerDissipation,
                    >::new(),
                    InspectRobotPropertyKindPlugin::<
                        InspectMechanicalSystem,
                        MechanicalSystem,
                        PowerDissipation,
                    >::new(),
                ),
            );
    }
}

/// Use this to create a minimal inspector plugin. You will be able to add your
/// own [`InspectionPlugin`]s to the application, but none of the standard
/// inspection plugins will be included.
#[derive(Default)]
pub struct MinimalInspectorPlugin {}

impl Plugin for MinimalInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MainInspector>();
    }
}

impl<W> InspectionPlugin<W>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W> Plugin for InspectionPlugin<W>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
{
    fn build(&self, app: &mut App) {
        let inspector = app.world().resource::<MainInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(app.world_mut());
        app.world_mut().spawn(widget).insert(ChildOf(inspector));
    }
}

/// This is the input type for inspection widgets. Use [`InspectionPlugin`] to
/// add the widget to the application.
#[derive(Clone, Copy)]
pub struct Inspect {
    /// What entity should be treated as selected.
    pub selection: Entity,
    /// What entity is the current inspection widget attached to.
    pub inspection: Entity,
    /// What kind of panel is the inspector rendered on.
    pub panel: PanelSettings,
}

/// This contains a reference to the main inspector widget of the application.
#[derive(Resource)]
pub struct MainInspector {
    id: Entity,
}

impl MainInspector {
    pub fn get(&self) -> Entity {
        self.id
    }
}

impl FromWorld for MainInspector {
    fn from_world(world: &mut World) -> Self {
        let widget = Widget::new::<Inspector>(world);
        let properties_panel = world.resource::<PropertiesPanel>().id();
        let id = world
            .spawn((widget, Name::new("Inspect")))
            .insert(ChildOf(properties_panel))
            .id();
        Self { id }
    }
}

/// Persistent widget-local state for the Site ID lookup control in the
/// [`Inspector`] widget.
#[derive(Default)]
struct SiteIdLookup {
    input: String,
    error: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum SiteIdLookupResult {
    Inspectable(Entity),
    NotInspectable,
    NotFound,
}

/// Finds the entity carrying `target` as its [`SiteID`], restricted to the
/// workspace rooted at `root` (i.e. `root` itself or one of its descendants).
fn find_entity_with_site_id(
    target: u32,
    root: Entity,
    site_ids: &Query<(Entity, &SiteID, Has<Selected>)>,
    child_of: &Query<&ChildOf>,
) -> SiteIdLookupResult {
    site_ids
        .iter()
        .find_map(|(entity, site_id, selectable)| {
            if site_id.0 != target {
                return None;
            }

            let in_workspace = entity == root
                || AncestorIter::new(child_of, entity).any(|ancestor| ancestor == root);
            in_workspace.then_some(if selectable {
                SiteIdLookupResult::Inspectable(entity)
            } else {
                SiteIdLookupResult::NotInspectable
            })
        })
        .unwrap_or(SiteIdLookupResult::NotFound)
}

/// Parses the user's Site ID input and resolves it to an entity that can be
/// selected, or returns a message explaining why it cannot.
fn resolve_site_id_input(
    input: &str,
    root: Option<Entity>,
    site_ids: &Query<(Entity, &SiteID, Has<Selected>)>,
    child_of: &Query<&ChildOf>,
) -> Result<Entity, String> {
    let target = input
        .trim()
        .parse::<u32>()
        .map_err(|_| "Site ID must be a whole number.".to_string())?;
    let root = root.ok_or_else(|| "No site is currently open.".to_string())?;
    match find_entity_with_site_id(target, root, site_ids, child_of) {
        SiteIdLookupResult::Inspectable(entity) => Ok(entity),
        SiteIdLookupResult::NotInspectable => Err(format!("Entity #{target} is not inspectable.")),
        SiteIdLookupResult::NotFound => Err(format!(
            "No entity with Site ID {target} was found in the current site."
        )),
    }
}

#[derive(SystemParam)]
pub struct Inspector<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
    inspect_for_query: Query<'w, 's, &'static InspectFor>,
    inspect_multi_selection: InspectMultiSelection<'w, 's>,
    current_workspace: Res<'w, CurrentWorkspace>,
    all_site_ids: Query<'w, 's, (Entity, &'static SiteID, Has<Selected>)>,
    child_of: Query<'w, 's, &'static ChildOf>,
    site_id_lookup: Local<'s, SiteIdLookup>,
}

impl<'w, 's> Inspector<'w, 's> {
    /// Renders a small control that lets a user type a raw numeric [`SiteID`]
    /// and select the matching entity, scoped to the current workspace.
    fn show_site_id_lookup(&mut self, ui: &mut Ui) {
        let mut submit = false;
        ui.horizontal(|ui| {
            ui.label("Site ID:");
            let response = ui.add(
                TextEdit::singleline(&mut self.site_id_lookup.input)
                    .desired_width(80.0)
                    .hint_text(RichText::new("e.g. 42").weak().italics()),
            );
            if response.changed() {
                self.site_id_lookup.error = None;
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                submit = true;
            }
            if ui.button("Select").clicked() {
                submit = true;
            }
        });

        if submit {
            self.site_id_lookup.error = match resolve_site_id_input(
                &self.site_id_lookup.input,
                self.current_workspace.root,
                &self.all_site_ids,
                &self.child_of,
            ) {
                Ok(entity) => {
                    self.inspect_multi_selection
                        .select
                        .write(Select::new(Some(entity)));
                    None
                }
                Err(error) => Some(error),
            };
        }

        if let Some(error) = &self.site_id_lookup.error {
            ui.colored_label(Color32::from_rgb(200, 60, 60), error);
        }

        ui.separator();
    }
}

impl<'w, 's> WidgetSystem<Tile> for Inspector<'w, 's> {
    fn show(
        Tile { id, panel }: Tile,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        // TODO(luca) make sure this doesn't show in building preview mode
        /*
        match world.resource::<State<AppState>>().get() {
            AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::WorkcellEditor => {}
            _ => return,
        }
        */

        // The Site ID lookup must stay visible even when nothing is selected,
        // so it is shown before the "no entity selected" early return below.
        state.get_mut(world).show_site_id_lookup(ui);

        let Some(selection) = world.get_resource::<Selection>() else {
            ui.label("ERROR: Selection resource is not available");
            return;
        };

        // Add prompts when no entity is selected to prevent tab from looking broken
        if selection.selected.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    bevy_egui::egui::RichText::new("No entity selected")
                        .italics()
                        .color(bevy_egui::egui::Color32::from_gray(160)),
                );
                ui.label("Click on an object in the scene to inspect its properties.");
            });
            return;
        }

        if selection.selected.len() > 1 {
            let instances: SmallVec<[Entity; 16]> = selection.selected.iter().cloned().collect();

            let mut inspect_multi_selection = state.get_mut(world).inspect_multi_selection;
            inspect_multi_selection.show_widget(instances, ui);
            return;
        }

        let Some(mut selection) = selection.get_single() else {
            return;
        };

        let inspect_for_query = state.get_mut(world).inspect_for_query;

        if let Ok(inspect_for) = inspect_for_query.get(selection) {
            selection = inspect_for.entity;
        }

        let params = state.get_mut(world);

        let (label, site_id) = if let Ok((category, site_id)) = params.heading.get(selection) {
            (
                category.map(|x| x.label()).unwrap_or("<Unknown Type>"),
                site_id,
            )
        } else {
            ("<Unknown Type>", None)
        };

        if let Some(site_id) = site_id {
            ui.heading(format!("{} #{}", label, site_id.0));
        } else {
            ui.heading(format!("{} (unsaved)", label));
        }

        let children: Result<SmallVec<[_; 16]>, _> = params
            .children
            .get(id)
            .map(|children| children.iter().collect());
        let Ok(children) = children else {
            return;
        };

        panel.align(ui, |ui| {
            for child in children {
                let inspect = Inspect {
                    selection,
                    inspection: child,
                    panel,
                };
                let _ = world.try_show_in(child, inspect, ui);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type LookupParams<'w, 's> = (
        Query<'w, 's, (Entity, &'static SiteID, Has<Selected>)>,
        Query<'w, 's, &'static ChildOf>,
    );

    fn resolve(world: &mut World, target: u32, root: Entity) -> SiteIdLookupResult {
        let mut state = SystemState::<LookupParams>::new(world);
        let (site_ids, child_of) = state.get(world);
        find_entity_with_site_id(target, root, &site_ids, &child_of)
    }

    fn resolve_input(
        world: &mut World,
        input: &str,
        root: Option<Entity>,
    ) -> Result<Entity, String> {
        let mut state = SystemState::<LookupParams>::new(world);
        let (site_ids, child_of) = state.get(world);
        resolve_site_id_input(input, root, &site_ids, &child_of)
    }

    #[test]
    fn finds_entity_in_current_workspace() {
        let mut world = World::new();
        let root = world.spawn((SiteID(1), Selected::default())).id();
        let child = world
            .spawn((SiteID(2), Selected::default(), ChildOf(root)))
            .id();

        assert_eq!(
            resolve(&mut world, 2, root),
            SiteIdLookupResult::Inspectable(child)
        );
        assert_eq!(
            resolve(&mut world, 1, root),
            SiteIdLookupResult::Inspectable(root)
        );
    }

    #[test]
    fn ignores_same_id_in_another_workspace() {
        let mut world = World::new();
        let root_a = world.spawn_empty().id();
        let root_b = world.spawn_empty().id();
        let child_a = world
            .spawn((SiteID(5), Selected::default(), ChildOf(root_a)))
            .id();
        let child_b = world
            .spawn((SiteID(5), Selected::default(), ChildOf(root_b)))
            .id();

        // Both workspaces contain an entity with SiteID(5); the lookup must
        // resolve to the one that actually belongs to the requested root.
        assert_eq!(
            resolve(&mut world, 5, root_a),
            SiteIdLookupResult::Inspectable(child_a)
        );
        assert_eq!(
            resolve(&mut world, 5, root_b),
            SiteIdLookupResult::Inspectable(child_b)
        );
    }

    #[test]
    fn handles_nested_descendants() {
        let mut world = World::new();
        let root = world.spawn(SiteID(1)).id();
        let level = world.spawn((SiteID(2), ChildOf(root))).id();
        let anchor = world
            .spawn((SiteID(3), Selected::default(), ChildOf(level)))
            .id();

        assert_eq!(
            resolve(&mut world, 3, root),
            SiteIdLookupResult::Inspectable(anchor)
        );
    }

    #[test]
    fn returns_not_found_for_missing_id() {
        let mut world = World::new();
        let root = world.spawn(SiteID(1)).id();
        world.spawn((SiteID(2), Selected::default(), ChildOf(root)));

        assert_eq!(resolve(&mut world, 99, root), SiteIdLookupResult::NotFound);
    }

    #[test]
    fn reports_entity_without_selected_as_not_inspectable() {
        let mut world = World::new();
        let root = world.spawn(SiteID(1)).id();
        world.spawn((SiteID(42), ChildOf(root)));

        assert_eq!(
            resolve(&mut world, 42, root),
            SiteIdLookupResult::NotInspectable
        );
        assert_eq!(
            resolve_input(&mut world, "42", Some(root)),
            Err("Entity #42 is not inspectable.".to_string())
        );
    }

    #[test]
    fn only_inspectable_input_resolves_to_a_selection() {
        let mut world = World::new();
        let root = world.spawn(SiteID(1)).id();
        let anchor = world
            .spawn((SiteID(3), Selected::default(), ChildOf(root)))
            .id();

        assert_eq!(resolve_input(&mut world, " 3 ", Some(root)), Ok(anchor));
        assert!(resolve_input(&mut world, "1", Some(root)).is_err());
        assert!(resolve_input(&mut world, "99", Some(root)).is_err());
        assert!(resolve_input(&mut world, "abc", Some(root)).is_err());
        assert!(resolve_input(&mut world, "3", None).is_err());
    }
}
