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

pub mod inspect_edge;
pub use inspect_edge::*;

pub mod inspect_fiducial;
pub use inspect_fiducial::*;

pub mod inspect_geography;
pub use inspect_geography::*;

pub mod inspect_group;
pub use inspect_group::*;

pub mod inspect_joint;
pub use inspect_joint::*;

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

pub mod inspect_mesh_constraint;
pub use inspect_mesh_constraint::*;

pub mod inspect_primitive_shape;
pub use inspect_primitive_shape::*;

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

pub mod inspect_scale;
pub use inspect_scale::*;

pub mod inspect_side;
pub use inspect_side::*;

pub mod inspect_texture;
pub use inspect_texture::*;

pub mod inspect_value;
pub use inspect_value::*;

pub mod inspect_workcell_parent;
pub use inspect_workcell_parent::*;

use crate::{
    interaction::{Selection, SpawnPreview},
    site::{
        AlignSiteDrawings, BeginEditDrawing, Category, Change, DefaultFile, DrawingMarker,
        EdgeLabels, LayerVisibility, Original, SiteID,
    },
    widgets::{AppEvents, SelectionWidget, prelude::*},
    AppState,
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*
};
use bevy_egui::egui::{Button, RichText, Ui, CollapsingHeader};
use rmf_site_format::*;
use smallvec::SmallVec;

#[derive(Default)]
pub struct StandardInspectorPlugin {

}

impl Plugin for StandardInspectorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ExInspectorWidget>()
            .add_plugins((
                InspectionPlugin::<ExInspectAnchor>::new(),
                InspectionPlugin::<InspectAnchorDependents>::new(),
                InspectionPlugin::<ExInspectEdge>::new(),
                InspectionPlugin::<InspectGeography>::new(),
                InspectionPlugin::<InspectFiducial>::new(),
                InspectionPlugin::<ExInspectLayer>::new(),
            ));
    }
}

pub struct InspectionPlugin<W>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
{
    _ignore: std::marker::PhantomData<W>,
}

impl<W> InspectionPlugin<W>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
{
    pub fn new() -> Self {
        Self { _ignore: Default::default() }
    }
}

impl<W> Plugin for InspectionPlugin<W>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
{
    fn build(&self, app: &mut App) {
        let inspector = app.world.resource::<ExInspectorWidget>().id;
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        app.world.spawn(widget).set_parent(inspector);
    }
}

#[derive(Clone, Copy)]
pub struct Inspect {
    pub selection: Entity,
    pub inspector: Entity,
    pub panel: PanelSide,
}

#[derive(Resource)]
pub struct ExInspectorWidget {
    id: Entity,
}

impl ExInspectorWidget {
    pub fn get(&self) -> Entity {
        self.id
    }
}

impl FromWorld for ExInspectorWidget {
    fn from_world(world: &mut World) -> Self {
        let widget = Widget::new::<ExInspectorWidgetParams>(world);
        let properties_panel = world.resource::<PropertiesPanel>().id;
        let id = world.spawn(widget).set_parent(properties_panel).id();
        Self { id }
    }
}

#[derive(SystemParam)]
struct ExInspectorWidgetParams<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
}

impl<'w, 's> WidgetSystem<Tile> for ExInspectorWidgetParams<'w, 's> {
    fn show(
        Tile{ id, panel }: Tile,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World
    ) {
        match world.resource::<State<AppState>>().get() {
            AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::WorkcellEditor => { }
            _ => return,
        }

        CollapsingHeader::new("Inspect")
            .default_open(true)
            .show(ui, |ui| {
                let Some(selection) = world.get_resource::<Selection>() else {
                    return;
                };

                let Some(selection) = selection.0 else {
                    return;
                };

                let params = state.get(world);

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

                let children: Result<SmallVec<[_; 16]>, _> = params.children
                    .get(id)
                    .map(|children| children.iter().copied().collect());
                let Ok(children) = children else {
                    return;
                };

                panel.align(ui, |ui| {
                    for child in children {
                        let inspect = Inspect { selection, inspector: child, panel };
                        let _ = world.try_show_in(child, inspect, ui);
                    }
                });
            });
    }
}

// Bevy seems to have a limit of 16 fields in a SystemParam struct, so we split
// some of the InspectorParams fields into the InspectorComponentParams struct.
#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub selection: Res<'w, Selection>,
    pub heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
    pub workcell_params: InspectorWorkcellParams<'w, 's>,
    pub component: InspectorComponentParams<'w, 's>,
    pub drawing: InspectDrawingParams<'w, 's>,
    // TODO(luca) move to new systemparam, reached 16 limit on main one
    pub primitive_shapes: Query<'w, 's, (&'static PrimitiveShape, &'static RecallPrimitiveShape)>,
    pub scales: Query<'w, 's, &'static Scale>,
    pub layer: InspectorLayerParams<'w, 's>,
    pub texture: InspectTextureAffiliationParams<'w, 's>,
    pub groups: InspectGroupParams<'w, 's>,
    pub default_file: Query<'w, 's, &'static DefaultFile>,
}

#[derive(SystemParam)]
pub struct InspectorWorkcellParams<'w, 's> {
    pub joints: InspectJointParams<'w, 's>,
    pub constraint_dependents_params: InspectModelDependentsParams<'w, 's>,
    pub names_in_workcell: Query<'w, 's, &'static NameInWorkcell>,
    pub workcell_names: Query<'w, 's, &'static NameOfWorkcell>,
    pub parent_params: InspectWorkcellParentParams<'w, 's>,
}

// NOTE: We may need to split this struct into multiple structs if we ever need
// it to have more than 16 fields.
#[derive(SystemParam)]
pub struct InspectorComponentParams<'w, 's> {
    pub edges: Query<
        'w,
        's,
        (
            &'static Edge<Entity>,
            Option<&'static Original<Edge<Entity>>>,
            Option<&'static EdgeLabels>,
            &'static Category,
        ),
    >,
    pub associated_graphs: InspectAssociatedGraphsParams<'w, 's>,
    pub location_tags: Query<'w, 's, (&'static LocationTags, &'static RecallLocationTags)>,
    pub motions: Query<'w, 's, (&'static Motion, &'static RecallMotion)>,
    pub reverse_motions: Query<'w, 's, (&'static ReverseLane, &'static RecallReverseLane)>,
    pub names: Query<'w, 's, &'static NameInSite>,
    pub doors: Query<'w, 's, (&'static DoorType, &'static RecallDoorType)>,
    pub lifts: InspectLiftParams<'w, 's>,
    pub poses: Query<'w, 's, &'static Pose>,
    pub asset_sources:
        Query<'w, 's, (&'static AssetSource, &'static RecallAssetSource), Without<Pending>>,
    pub constraint_dependents: Query<'w, 's, With<ConstraintDependents>>,
    pub pixels_per_meter: Query<'w, 's, &'static PixelsPerMeter>,
    pub physical_camera_properties: Query<'w, 's, &'static PhysicalCameraProperties>,
    pub lights: Query<'w, 's, (&'static LightKind, &'static RecallLightKind)>,
    pub previewable: Query<'w, 's, &'static PreviewableMarker>,
}

#[derive(SystemParam)]
pub struct InspectDrawingParams<'w, 's> {
    pub distance: Query<'w, 's, &'static Distance>,
}

#[derive(SystemParam)]
pub struct InspectorLayerParams<'w, 's> {
    pub floors: Query<
        'w,
        's,
        (
            Option<&'static LayerVisibility>,
            &'static PreferredSemiTransparency,
        ),
        With<FloorMarker>,
    >,
    pub drawings: Query<
        'w,
        's,
        (
            Option<&'static LayerVisibility>,
            &'static PreferredSemiTransparency,
        ),
        With<DrawingMarker>,
    >,
    pub levels: Query<
        'w,
        's,
        (
            &'static GlobalFloorVisibility,
            &'static GlobalDrawingVisibility,
        ),
    >,
}

pub struct InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub params: &'a InspectorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(params: &'a InspectorParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    fn heading(&self, selection: Entity, ui: &mut Ui) {
        let (label, site_id) = if let Ok((category, site_id)) = self.params.heading.get(selection) {
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
    }

    pub fn show(mut self, ui: &mut Ui) {
        let default_file = self
            .events
            .request
            .current_workspace
            .root
            .map(|e| self.params.default_file.get(e).ok())
            .flatten();

        if let Some(selection) = self.params.selection.0 {
            self.heading(selection, ui);

            if let Ok(name) = self.params.component.names.get(selection) {
                if let Some(new_name) = InspectName::new(name).show(ui) {
                    self.events
                        .change
                        .name
                        .send(Change::new(new_name, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(name) = self.params.workcell_params.names_in_workcell.get(selection) {
                if let Some(new_name) = InspectNameInWorkcell::new(name).show(ui) {
                    self.events
                        .workcell_change
                        .name_in_workcell
                        .send(Change::new(new_name, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(name) = self.params.workcell_params.workcell_names.get(selection) {
                if let Some(new_name) = InspectNameOfWorkcell::new(name).show(ui) {
                    self.events
                        .workcell_change
                        .workcell_name
                        .send(Change::new(new_name, selection));
                }
                ui.add_space(10.0);
            }

            // if let Ok((floor_vis, alpha)) = self.params.layer.floors.get(selection) {
            //     ui.horizontal(|ui| {
            //         MoveLayer::new(
            //             selection,
            //             &mut self.events.layers.floors,
            //             &self.events.layers.icons,
            //         )
            //         .show(ui);
            //     });
            //     ui.horizontal(|ui| {
            //         InspectLayer::new(
            //             selection,
            //             &self.params.anchor_params.icons,
            //             self.events,
            //             floor_vis.copied(),
            //             alpha.0,
            //             true,
            //         )
            //         .show(ui);
            //     });
            // }

            // if let Ok((drawing_vis, alpha)) = self.params.layer.drawings.get(selection) {
            //     ui.horizontal(|ui| {
            //         MoveLayer::new(
            //             selection,
            //             &mut self.events.layers.drawings,
            //             &self.events.layers.icons,
            //         )
            //         .show(ui);
            //     });
            //     ui.horizontal(|ui| {
            //         InspectLayer::new(
            //             selection,
            //             &self.params.anchor_params.icons,
            //             self.events,
            //             drawing_vis.copied(),
            //             alpha.0,
            //             false,
            //         )
            //         .show(ui);
            //     });
            // }

            if let Ok(ppm) = self.params.component.pixels_per_meter.get(selection) {
                if *self.events.app_state.get() == AppState::SiteEditor {
                    ui.add_space(10.0);
                    if ui
                        .add(Button::image_and_text(
                            self.events.layers.icons.edit.egui(),
                            "Edit Drawing",
                        ))
                        .clicked()
                    {
                        self.events
                            .layers
                            .begin_edit_drawing
                            .send(BeginEditDrawing(selection));
                    }
                }
                ui.add_space(10.0);
                if ui
                    .add(Button::image_and_text(
                        self.events.layers.icons.alignment.egui(),
                        "Align Drawings",
                    ))
                    .on_hover_text(
                        "Align all drawings in the site based on their fiducials and measurements",
                    )
                    .clicked()
                {
                    if let Some(site) = self.events.request.current_workspace.root {
                        self.events.request.align_site.send(AlignSiteDrawings(site));
                    }
                }
                ui.add_space(10.0);
                if let Some(new_ppm) =
                    InspectValue::<f32>::new(String::from("Pixels per meter"), ppm.0)
                        .clamp_range(0.0001..=std::f32::INFINITY)
                        .tooltip("How many image pixels per meter".to_string())
                        .show(ui)
                {
                    self.events
                        .change
                        .pixels_per_meter
                        .send(Change::new(PixelsPerMeter(new_ppm), selection));
                }
            }

            InspectAssociatedGraphsWidget::new(
                selection,
                &self.params.component.associated_graphs,
                self.events,
            )
            .show(ui);

            // if let Ok((tags, recall)) = self.params.component.location_tags.get(selection) {
            //     if let Some(new_tags) = InspectLocationWidget::new(
            //         selection,
            //         tags,
            //         recall,
            //         &self.params.anchor_params.icons,
            //         self.events,
            //     )
            //     .show(ui)
            //     {
            //         self.events
            //             .change
            //             .location_tags
            //             .send(Change::new(new_tags, selection));
            //     }
            // }

            InspectTextureAffiliation::new(selection, &self.params.texture, self.events).show(ui);

            if let Ok((motion, recall)) = self.params.component.motions.get(selection) {
                ui.label(RichText::new("Forward Motion").size(18.0));
                if let Some(new_motion) = InspectMotionWidget::new(motion, recall).show(ui) {
                    self.events
                        .change
                        .lane_motion
                        .send(Change::new(new_motion, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((reverse, recall)) = self.params.component.reverse_motions.get(selection) {
                ui.separator();
                ui.push_id("Reverse Motion", |ui| {
                    if let Some(new_reverse) = InspectReverseWidget::new(reverse, recall).show(ui) {
                        self.events
                            .change
                            .lane_reverse
                            .send(Change::new(new_reverse, selection));
                    }
                });
                ui.add_space(10.0);
            }

            if let Ok(pose) = self.params.component.poses.get(selection) {
                if let Some(new_pose) = InspectPose::new(pose).show(ui) {
                    self.events
                        .change
                        .pose
                        .send(Change::new(new_pose, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(scale) = self.params.scales.get(selection) {
                if let Some(new_scale) = InspectScale::new(scale).show(ui) {
                    self.events
                        .workcell_change
                        .scale
                        .send(Change::new(new_scale, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((light, recall)) = self.params.component.lights.get(selection) {
                if let Some(new_light) = InspectLightKind::new(light, recall).show(ui) {
                    self.events
                        .change
                        .light
                        .send(Change::new(new_light, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((door, recall)) = self.params.component.doors.get(selection) {
                if let Some(new_door) = InspectDoorType::new(door, recall).show(ui) {
                    self.events
                        .change
                        .door
                        .send(Change::new(new_door, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((source, recall)) = self.params.component.asset_sources.get(selection) {
                if let Some(new_asset_source) =
                    InspectAssetSource::new(source, recall, default_file).show(ui)
                {
                    self.events
                        .change
                        .asset_source
                        .send(Change::new(new_asset_source, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((source, recall)) = self.params.primitive_shapes.get(selection) {
                if let Some(new_primitive_shape) =
                    InspectPrimitiveShape::new(source, recall).show(ui)
                {
                    self.events
                        .workcell_change
                        .primitive_shapes
                        .send(Change::new(new_primitive_shape, selection));
                }
                ui.add_space(10.0);
            }

            if self
                .params
                .component
                .constraint_dependents
                .get(selection)
                .is_ok()
            {
                InspectModelDependentsWidget::new(
                    selection,
                    &self.params.workcell_params.constraint_dependents_params,
                    self.events,
                )
                .show(ui);
                ui.add_space(10.0);
            }

            InspectWorkcellParentWidget::new(
                selection,
                &self.params.workcell_params.parent_params,
                &mut self.events,
            )
            .show(ui);
            InspectJointWidget::new(
                selection,
                &self.params.workcell_params.joints,
                &mut self.events,
            )
            .show(ui);

            if let Ok(distance) = self.params.drawing.distance.get(selection) {
                if let Some(new_distance) =
                    InspectOptionF32::new("Distance".to_string(), distance.0, 10.0)
                        .clamp_range(0.0..=10000.0)
                        .min_decimals(2)
                        .max_decimals(2)
                        .speed(0.01)
                        .suffix(" m".to_string())
                        .show(ui)
                {
                    self.events
                        .change
                        .distance
                        .send(Change::new(Distance(new_distance), selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(camera_properties) = self
                .params
                .component
                .physical_camera_properties
                .get(selection)
            {
                if let Some(new_camera_properties) =
                    InspectPhysicalCameraProperties::new(camera_properties).show(ui)
                {
                    self.events
                        .change
                        .physical_camera_properties
                        .send(Change::new(new_camera_properties, selection));
                }
                ui.add_space(10.0);
            }

            if let Some(new_cabin) =
                InspectLiftCabin::new(selection, &self.params.component.lifts, &mut self.events)
                    .show(ui)
            {
                self.events
                    .change
                    .lift_cabin
                    .send(Change::new(new_cabin, selection));
                ui.add_space(10.0);
            }

            if let Ok(_previewable) = self.params.component.previewable.get(selection) {
                if ui.button("Preview").clicked() {
                    self.events
                        .request
                        .spawn_preview
                        .send(SpawnPreview::new(Some(selection)));
                }
                ui.add_space(10.0);
            }

            if let Ok(Affiliation(Some(group))) = self.params.groups.affiliation.get(selection) {
                ui.separator();
                let empty = String::new();
                let name = self
                    .params
                    .component
                    .names
                    .get(*group)
                    .map(|n| &n.0)
                    .unwrap_or(&empty);

                ui.label(RichText::new(format!("Group Properties of [{}]", name)).size(18.0));
                ui.add_space(5.0);
                InspectGroup::new(
                    *group,
                    selection,
                    default_file,
                    &self.params.groups,
                    self.events,
                )
                .show(ui);
            }

            if self.params.groups.is_group.contains(selection) {
                InspectGroup::new(
                    selection,
                    selection,
                    default_file,
                    &self.params.groups,
                    self.events,
                )
                .show(ui);
            }
        } else {
            ui.label("Nothing selected");
        }
    }
}
