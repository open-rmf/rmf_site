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
use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(SystemParam)]
pub struct GraphSelect<'w, 's> {
    graphs: Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            &'static Visibility,
            &'static RecencyRank<NavGraphMarker>,
        ),
        With<NavGraphMarker>,
    >,
    assets: Res<'w, SiteAssets>,
}

impl<'w, 's> GraphSelect<'w, 's> {
    // TODO(MXG): In the future we should consider using StandardMaterial's
    // depth_bias to fix issues with rendering overlapping flat objects. That
    // will have to wait until a future release where https://github.com/bevyengine/bevy/pull/7847
    // has been merged. It will also require the picking algorithm to be
    // sensitive to ranks.
    pub fn display_style(
        &self,
        associated_graphs: &AssociatedGraphs,
    ) -> (Handle<StandardMaterial>, f32) {
        match associated_graphs {
            AssociatedGraphs::All => self
                .graphs
                .iter()
                .filter(|(_, _, v, _)| !matches!(v, Visibility::Hidden))
                .max_by(|(_, _, _, a), (_, _, _, b)| a.cmp(b))
                .map(|(_, m, _, d)| (m.clone(), *d)),
            AssociatedGraphs::Only(set) => set
                .iter()
                .filter(|e| {
                    self.graphs
                        .get(***e)
                        .ok()
                        .filter(|(_, _, v, _)| !matches!(v, Visibility::Hidden))
                        .is_some()
                })
                .max_by(|a, b| {
                    self.graphs
                        .get(***a)
                        .unwrap()
                        .3
                        .cmp(self.graphs.get(***b).unwrap().3)
                })
                .map(|e| self.graphs.get(**e).map(|(_, m, _, d)| (m.clone(), *d)).ok())
                .flatten(),
            AssociatedGraphs::AllExcept(set) => self
                .graphs
                .iter()
                .filter(|(e, _, v, _)| !matches!(v, Visibility::Hidden) && !set.contains(e))
                .max_by(|(_, _, _, a), (_, _, _, b)| a.cmp(b))
                .map(|(_, m, _, d)| (m.clone(), *d)),
        }
        .map(|(m, d)| {
            (
                m.0,
                d.proportion() * (LANE_LAYER_LIMIT - LANE_LAYER_START) + LANE_LAYER_START,
            )
        })
        .unwrap_or((
            self.assets.unassigned_lane_material.clone(),
            LANE_LAYER_LIMIT,
        ))
    }

    pub fn should_display(&self, associated_graphs: &AssociatedGraphs) -> bool {
        match associated_graphs {
            AssociatedGraphs::All => {
                self.graphs.is_empty()
                    || self
                        .graphs
                        .iter()
                        .find(|(_, _, v, _)| !matches!(v, Visibility::Hidden))
                        .is_some()
            }
            AssociatedGraphs::Only(set) => {
                self.graphs.is_empty()
                    || set.is_empty()
                    || set
                        .iter()
                        .find(|e| {
                            self.graphs
                                .get(***e)
                                .ok()
                                .filter(|(_, _, v, _)| !matches!(v, Visibility::Hidden))
                                .is_some()
                        })
                        .is_some()
            }
            AssociatedGraphs::AllExcept(set) => {
                self.graphs.iter().find(|(e, _, v, _)| !matches!(v, Visibility::Hidden) && !set.contains(e)).is_some()
                // If all graphs are excluded for this lane then we want it to remain
                // visible but with the unassigned material
                || self.graphs.iter().find(|(e, _, _, _)| !set.contains(e)).is_none()
            }
        }
    }
}

pub fn add_category_to_graphs(
    mut commands: Commands,
    new_graphs: Query<Entity, (With<NavGraphMarker>, Without<Category>)>,
) {
    for e in &new_graphs {
        commands.entity(e).insert(Category::NavigationGraph);
    }
}
