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

pub mod inspect_f32;
pub use inspect_f32::*;

pub mod inspect_is_static;
pub use inspect_is_static::*;

pub mod inspect_option_string;
pub use inspect_option_string::*;

pub mod inspect_lane;
pub use inspect_lane::*;

pub mod inspect_name;
pub use inspect_name::*;

pub mod inspect_option_f32;
pub use inspect_option_f32::*;

pub mod inspect_pose;
pub use inspect_pose::*;

pub mod inspect_side;
pub use inspect_side::*;

pub mod selection_widget;
pub use selection_widget::*;

use crate::{
    interaction::Selection,
    site::{Category, Change, EdgeLabels, Original, SiteID},
    widgets::AppEvents,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{RichText, Ui};
use rmf_site_format::*;

#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub selection: Res<'w, Selection>,
    pub heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
    pub anchor_params: InspectAnchorParams<'w, 's>,
    pub anchor_dependents_params: InspectAnchorDependentsParams<'w, 's>,
    pub edges: Query<
        'w,
        's,
        (
            &'static Edge<Entity>,
            Option<&'static Original<Edge<Entity>>>,
            Option<&'static EdgeLabels>,
        ),
    >,
    pub motions: Query<'w, 's, (&'static Motion, &'static RecallMotion)>,
    pub reverse_motions: Query<'w, 's, (&'static ReverseLane, &'static RecallReverseLane)>,
    pub names: Query<'w, 's, &'static NameInSite>,
    pub kinds: Query<'w, 's, (&'static Kind, &'static RecallKind)>,
    pub labels: Query<'w, 's, (&'static Label, &'static RecallLabel)>,
    pub doors: Query<'w, 's, (&'static DoorType, &'static RecallDoorType)>,
    pub poses: Query<'w, 's, &'static Pose>,
    pub asset_sources: Query<'w, 's, (&'static AssetSource, &'static RecallAssetSource)>,
    pub pixels_per_meters: Query<'w, 's, &'static PixelsPerMeter>,
}

pub struct InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub params: &'a mut InspectorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        params: &'a mut InspectorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self { params, events }
    }

    fn heading(&self, selection: Entity, ui: &mut Ui) {
        let (label, site_id) = if let Ok((category, site_id)) = self.params.heading.get(selection) {
            (
                category.map(|x| x.0.as_str()).unwrap_or("<Unknown Type>"),
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

    pub fn show(self, ui: &mut Ui) {
        if let Some(selection) = self.params.selection.0 {
            self.heading(selection, ui);
            if self.params.anchor_params.transforms.contains(selection) {
                ui.horizontal(|ui| {
                    InspectAnchorWidget::new(
                        selection,
                        &mut self.params.anchor_params,
                        self.events,
                    )
                    .show(ui);
                });
                ui.separator();
                InspectAnchorDependentsWidget::new(
                    selection,
                    &mut self.params.anchor_dependents_params,
                    self.events,
                )
                .show(ui);
                ui.add_space(10.0);
            }

            if let Ok((edge, original, labels)) = self.params.edges.get(selection) {
                InspectEdgeWidget::new(
                    selection,
                    edge,
                    original,
                    labels,
                    &mut self.params.anchor_params,
                    self.events,
                )
                .show(ui);
                ui.add_space(10.0);
            }

            if let Ok((motion, recall)) = self.params.motions.get(selection) {
                ui.label(RichText::new("Forward Motion").size(18.0));
                if let Some(new_motion) = InspectMotionWidget::new(motion, recall).show(ui) {
                    self.events
                        .change_lane_motion
                        .send(Change::new(new_motion, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((reverse, recall)) = self.params.reverse_motions.get(selection) {
                ui.separator();
                ui.push_id("Reverse Motion", |ui| {
                    if let Some(new_reverse) = InspectReverseWidget::new(reverse, recall).show(ui) {
                        self.events
                            .change_lane_reverse
                            .send(Change::new(new_reverse, selection));
                    }
                });
                ui.add_space(10.0);
            }

            if let Ok(name) = self.params.names.get(selection) {
                if let Some(new_name) = InspectName::new(name).show(ui) {
                    self.events
                        .change_name
                        .send(Change::new(new_name, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((kind, recall)) = self.params.kinds.get(selection) {
                if let Some(new_kind) =
                    InspectOptionString::new("Kind", &kind.0, &recall.value).show(ui)
                {
                    self.events
                        .change_kind
                        .send(Change::new(Kind(new_kind), selection));
                }
            }

            if let Ok((label, recall)) = self.params.labels.get(selection) {
                if let Some(new_label) =
                    InspectOptionString::new("Label", &label.0, &recall.value).show(ui)
                {
                    self.events
                        .change_label
                        .send(Change::new(Label(new_label), selection));
                }
            }

            if let Ok(pose) = self.params.poses.get(selection) {
                if let Some(new_pose) = InspectPose::new(pose).show(ui) {
                    self.events
                        .change_pose
                        .send(Change::new(new_pose, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((door, recall)) = self.params.doors.get(selection) {
                if let Some(new_door) = InspectDoorType::new(door, recall).show(ui) {
                    self.events
                        .change_door
                        .send(Change::new(new_door, selection));
                }
            }

            if let Ok((source, recall)) = self.params.asset_sources.get(selection) {
                if let Some(new_asset_source) = InspectAssetSource::new(source, recall).show(ui) {
                    self.events
                        .change_asset_source
                        .send(Change::new(new_asset_source, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok(ppm) = self.params.pixels_per_meters.get(selection) {
                if let Some(new_ppm) = InspectF32::new(String::from("Pixels per meter"), ppm.0)
                    .clamp_range(0.0..=std::f32::INFINITY)
                    .tooltip("How many image pixels per meter".to_string())
                    .show(ui)
                {
                    self.events
                        .change_pixels_per_meter
                        .send(Change::new(PixelsPerMeter(new_ppm), selection));
                }
                ui.add_space(10.0);
            }
        } else {
            ui.label("Nothing selected");
        }
    }
}
