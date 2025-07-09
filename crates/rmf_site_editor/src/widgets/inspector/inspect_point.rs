/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use crate::{
    interaction::{AnchorScope, AnchorSelection},
    site::{DrawingMarker, Original, SiteID},
    widgets::{
        inspector::{Inspect, InspectAnchor, InspectAnchorInput},
        prelude::*,
    },
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use bevy_egui::egui::{Grid, Ui};
use rmf_site_egui::*;
use rmf_site_format::{NameOfSite, Point};

#[derive(SystemParam)]
pub struct InspectPoint<'w, 's> {
    points: Query<
        'w,
        's,
        (
            &'static ChildOf,
            &'static Point<Entity>,
            Option<&'static Original<Point<Entity>>>,
        ),
    >,
    scopes: Query<'w, 's, (Option<&'static NameOfSite>, Option<&'static DrawingMarker>)>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> ShareableWidget for InspectPoint<'w, 's> {}

impl<'w, 's> WidgetSystem<Inspect> for InspectPoint<'w, 's> {
    fn show(
        Inspect {
            selection: id,
            panel,
            ..
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let params = state.get_mut(world);
        let Ok((child_of, current_point, original)) = params.points.get(id) else {
            return;
        };

        let point = match original {
            Some(original) => original.0,
            None => *current_point,
        };

        let parent = child_of.parent();
        let scope = match params.scopes.get(parent) {
            Ok((site, drawing)) => {
                if site.is_some() {
                    AnchorScope::Site
                } else if drawing.is_some() {
                    AnchorScope::Drawing
                } else {
                    AnchorScope::General
                }
            }
            Err(_) => AnchorScope::General,
        };

        let anchor = point.0;
        Grid::new("inspect_point").show(ui, |ui| {
            ui.label("ID");
            ui.label("");
            ui.label("x");
            ui.label("y");
            ui.end_row();

            let response = world.show::<InspectAnchor, _, _>(
                InspectAnchorInput {
                    anchor,
                    is_dependency: true,
                    panel,
                },
                ui,
            );
            ui.end_row();

            match response {
                Some(response) => {
                    if response.replace {
                        let mut params = state.get_mut(world);
                        params.anchor_selection.replace_point(id, scope);
                        info!("Triggered anchor replacement for point {id:?}");
                    }
                }
                None => {
                    error!(
                        "The reference anchor for point {id:?} (Site ID {:?}) is not an \
                        anchor: {anchor:?}! This should never happen! Please report \
                        this to the site editor developers.",
                        world.get::<SiteID>(anchor),
                    );
                }
            }
        });
    }
}
