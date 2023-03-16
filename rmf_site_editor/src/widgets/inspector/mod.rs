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

pub mod inspect_side;
pub use inspect_side::*;

pub mod inspect_value;
pub use inspect_value::*;

pub mod selection_widget;
pub use selection_widget::*;

use crate::{
    interaction::{Selection, SpawnPreview},
    site::{Category, Change, EdgeLabels, FloorVisibility, Original, SiteID},
    widgets::AppEvents,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{RichText, Ui};
use rmf_site_format::*;

// Bevy seems to have a limit of 16 fields in a SystemParam struct, so we split
// some of the InspectorParams fields into the InspectorComponentParams struct.
#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub selection: Res<'w, Selection>,
    pub heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
    pub anchor_params: InspectAnchorParams<'w, 's>,
    pub anchor_dependents_params: InspectAnchorDependentsParams<'w, 's>,
    pub component: InspectorComponentParams<'w, 's>,
    pub layer: InspectorLayerParams<'w, 's>,
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
    pub labels: Query<'w, 's, (&'static Label, &'static RecallLabel)>,
    pub doors: Query<'w, 's, (&'static DoorType, &'static RecallDoorType)>,
    pub lifts: InspectLiftParams<'w, 's>,
    pub poses: Query<'w, 's, &'static Pose>,
    pub asset_sources: Query<'w, 's, (&'static AssetSource, &'static RecallAssetSource)>,
    pub pixels_per_meter: Query<'w, 's, &'static PixelsPerMeter>,
    pub physical_camera_properties: Query<'w, 's, &'static PhysicalCameraProperties>,
    pub lights: Query<'w, 's, (&'static LightKind, &'static RecallLightKind)>,
    pub previewable: Query<'w, 's, &'static PreviewableMarker>,
}

#[derive(SystemParam)]
pub struct InspectorLayerParams<'w, 's> {
    pub floors: Query<'w, 's, Option<&'static FloorVisibility>, With<FloorMarker>>,
    pub drawings: Query<'w, 's, (), With<DrawingMarker>>,
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
        if let Some(selection) = self.params.selection.0 {
            self.heading(selection, ui);
            if self.params.anchor_params.anchors.contains(selection) {
                ui.horizontal(|ui| {
                    InspectAnchorWidget::new(selection, &self.params.anchor_params, self.events)
                        .show(ui);
                });
                ui.separator();
                InspectAnchorDependentsWidget::new(
                    selection,
                    &self.params.anchor_dependents_params,
                    self.events,
                )
                .show(ui);
                ui.add_space(10.0);
            }

            if let Ok(name) = self.params.component.names.get(selection) {
                if let Some(new_name) = InspectName::new(name).show(ui) {
                    self.events
                        .change
                        .name
                        .send(Change::new(new_name, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(floor_vis) = self.params.layer.floors.get(selection) {
                ui.horizontal(|ui| {
                    InspectLayer::new(selection, &self.params.anchor_params.icons, self.events)
                        .as_floor(floor_vis.copied())
                        .show(ui);
                });
            }

            if self.params.layer.drawings.contains(selection) {
                ui.horizontal(|ui| {
                    InspectLayer::new(selection, &self.params.anchor_params.icons, self.events)
                        .show(ui);
                });
            }

            if let Ok((edge, original, labels, category)) =
                self.params.component.edges.get(selection)
            {
                InspectEdgeWidget::new(
                    selection,
                    category,
                    edge,
                    original,
                    labels,
                    &self.params.anchor_params,
                    self.events,
                )
                .show(ui);
                ui.add_space(10.0);
            }

            InspectAssociatedGraphsWidget::new(
                selection,
                &self.params.component.associated_graphs,
                self.events,
            )
            .show(ui);

            if let Ok((tags, recall)) = self.params.component.location_tags.get(selection) {
                if let Some(new_tags) = InspectLocationWidget::new(
                    selection,
                    tags,
                    recall,
                    &self.params.anchor_params.icons,
                    self.events,
                )
                .show(ui)
                {
                    self.events
                        .change
                        .location_tags
                        .send(Change::new(new_tags, selection));
                }
            }

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

            if let Ok((label, recall)) = self.params.component.labels.get(selection) {
                if let Some(new_label) =
                    InspectOptionString::new("Label", &label.0, &recall.value).show(ui)
                {
                    self.events
                        .change
                        .label
                        .send(Change::new(Label(new_label), selection));
                }
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
                if let Some(new_asset_source) = InspectAssetSource::new(source, recall).show(ui) {
                    self.events
                        .change
                        .asset_source
                        .send(Change::new(new_asset_source, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(ppm) = self.params.component.pixels_per_meter.get(selection) {
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
        } else {
            ui.label("Nothing selected");
        }
    }
}
