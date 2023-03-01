/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::site::*;
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};

#[derive(SystemParam)]
pub struct GraphSelect<'w, 's> {
    graphs: Query<'w, 's, (Entity, &'static Handle<StandardMaterial>, &'static Visibility, &'static DisplayLayer), With<NavGraphMarker>>,
    assets: Res<'w, SiteAssets>,
}

impl<'w, 's> GraphSelect<'w, 's> {
    pub fn pick_material(&self, associated_graphs: &AssociatedGraphs<Entity>) -> Handle<StandardMaterial> {
        match associated_graphs {
            AssociatedGraphs::All => self.graphs
                .iter()
                .filter(|(_, _, v, _)| v.is_visible)
                .max_by(|(_, _, _, a), (_, _, _, b)| a.cmp(b))
                .map(|(_, m, _, _)| m)
                .unwrap_or(&self.assets.unassigned_lane_material)
                .clone(),
            AssociatedGraphs::Only(set) => set
                .iter()
                .filter(|e| {
                    self.graphs
                        .get(**e)
                        .ok()
                        .filter(|(_, _, v, _)| v.is_visible)
                        .is_some()
                })
                .max_by(|a, b| self.graphs.get(**a).unwrap().3.cmp(self.graphs.get(**b).unwrap().3))
                .map(|e| self.graphs.get(*e).map(|(_, m, _, _)| m).ok())
                .flatten()
                .unwrap_or(&self.assets.unassigned_lane_material)
                .clone(),
            AssociatedGraphs::AllExcept(set) => self.graphs
                .iter()
                .filter(|(e, _, v, _)| v.is_visible && !set.contains(e))
                .max_by(|(_, _, _, a), (_, _, _, b)| a.cmp(b))
                .map(|(_, m, _, _)| m)
                .unwrap_or(&self.assets.unassigned_lane_material)
                .clone(),
        }
    }

    pub fn should_display(&self, associated_graphs: &AssociatedGraphs<Entity>) -> bool {
        match associated_graphs {
            AssociatedGraphs::All => {
                self.graphs.is_empty() || self.graphs.iter().find(|(_, _, v, _)| v.is_visible).is_some()
            }
            AssociatedGraphs::Only(set) => {
                self.graphs.is_empty()
                    || set.is_empty()
                    || set
                        .iter()
                        .find(|e| self.graphs.get(**e).ok().filter(|(_, _, v, _)| v.is_visible).is_some())
                        .is_some()
            }
            AssociatedGraphs::AllExcept(set) => {
                self.graphs.iter().find(|(e, _, v, _)| v.is_visible && !set.contains(e)).is_some()
                // If all graphs are excluded for this lane then we want it to remain
                // visible but with the unassigned material
                || self.graphs.iter().find(|(e, _, _, _)| !set.contains(e)).is_none()
            }
        }
    }
}
