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

pub mod inspect_is_static;
pub use inspect_is_static::*;

pub mod inspect_option_string;
pub use inspect_option_string::*;

pub mod inspect_layer;
pub use inspect_layer::*;

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

pub mod inspect_task;
pub use inspect_task::*;

pub mod inspect_texture;
pub use inspect_texture::*;

pub mod inspect_value;
pub use inspect_value::*;

use crate::{
    interaction::Selection,
    site::{Category, SiteID},
    widgets::prelude::*,
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, Ui};
use rmf_site_format::*;
use smallvec::SmallVec;

/// Use this plugin to add a single inspection tile into the [`MainInspector`]
/// widget.
///
/// ```no_run
/// use bevy::prelude::{App, Query, Entity, Res};
/// use librmf_site_editor::{SiteEditor, site::NameInSite, widgets::prelude::*};
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
                InspectionPlugin::<InspectAssociatedGraphs>::new(),
                InspectionPlugin::<InspectLocation>::new(),
                InspectTexturePlugin::default(),
                InspectionPlugin::<InspectMotion>::new(),
                InspectionPlugin::<InspectPose>::new(),
                // Reached the tuple limit
            ))
            .add_plugins((
                InspectionPlugin::<InspectScale>::new(),
                InspectionPlugin::<InspectLight>::new(),
                InspectionPlugin::<InspectDoor>::new(),
                InspectionPlugin::<InspectPrimitiveShape>::new(),
                InspectionPlugin::<InspectMeasurement>::new(),
                InspectionPlugin::<InspectPhysicalCameraProperties>::new(),
                InspectionPlugin::<InspectPreview>::new(),
                InspectionPlugin::<InspectGroup>::new(),
                InspectModelDescriptionPlugin::default(),
                InspectLiftPlugin::default(),
            ))
            .add_plugins((
                // Required model properties
                InspectModelPropertyPlugin::<InspectModelScale, Scale>::new("Scale".to_string()),
                InspectModelPropertyPlugin::<InspectModelAssetSource, AssetSource>::new(
                    "Asset Source".to_string(),
                ),
                // Optional model properties
                InspectMobilityPlugin::default(),
                InspectDifferentialDrivePlugin::default(),
                InspectTaskPlugin::default(),
            ));
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
        let inspector = app.world.resource::<MainInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        app.world.spawn(widget).set_parent(inspector);
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
    pub panel: PanelSide,
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
        let id = world.spawn(widget).set_parent(properties_panel).id();
        Self { id }
    }
}

#[derive(SystemParam)]
pub struct Inspector<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
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

        CollapsingHeader::new("Inspect")
            .default_open(true)
            .show(ui, |ui| {
                let Some(selection) = world.get_resource::<Selection>() else {
                    ui.label("ERROR: Selection resource is not available");
                    return;
                };

                let Some(selection) = selection.0 else {
                    ui.label("Nothing selected");
                    return;
                };

                let params = state.get(world);

                let (label, site_id) =
                    if let Ok((category, site_id)) = params.heading.get(selection) {
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
                    .map(|children| children.iter().copied().collect());
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
            });
    }
}
