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

use crate::{
    interaction::*,
    site::{CollisionMeshMarker, DrawingMarker, VisualMeshMarker},
};
use bevy::color::palettes::css as Colors;
use bevy::render::view::RenderLayers;
use bevy_mod_outline::{
    ComputedOutline, InheritOutline, OutlineMode, OutlineRenderLayers, OutlineStencil,
    OutlineVolume,
};
use rmf_site_camera::*;
use rmf_site_format::{
    ConveyorMarker, DoorType, FiducialMarker, FloorMarker, LiftCabin, LightKind, LocationTags,
    MeasurementMarker, ModelMarker, PhysicalCameraProperties, PrimitiveShape, WallMarker,
};
use smallvec::SmallVec;

// TODO(MXG): Customize the behavior of floor, wall, and model visual cues.
// For now we just use the same interaction behavior for all of them.
#[derive(Component)]
pub enum OutlineVisualization {
    Ordinary,
    Silhouette,
    Anchor { body: Entity },
}

impl Default for OutlineVisualization {
    fn default() -> Self {
        OutlineVisualization::Ordinary
    }
}

impl OutlineVisualization {
    pub fn color(&self, hovered: &Hovered, selected: &Selected) -> Option<Color> {
        match self {
            OutlineVisualization::Ordinary | OutlineVisualization::Silhouette => {
                if !hovered.cue() && !selected.cue() {
                    None
                } else if hovered.cue() && selected.cue() {
                    Some(Color::srgb(1.0, 0.0, 0.3))
                } else if selected.cue() {
                    Some(Color::srgb(1.0, 0.3, 1.0))
                } else
                /* only hovered */
                {
                    Some(Colors::WHITE.into())
                }
            }
            OutlineVisualization::Anchor { .. } => {
                if hovered.is_hovered {
                    Some(Colors::WHITE.into())
                } else {
                    Some(Colors::BLACK.into())
                }
            }
        }
    }

    pub fn layers(
        &self,
        hovered: &Hovered,
        selected: &Selected,
        layer: Option<&RenderLayers>,
    ) -> OutlineRenderLayers {
        match self {
            OutlineVisualization::Ordinary | OutlineVisualization::Silhouette => {
                if hovered.cue() {
                    OutlineRenderLayers(RenderLayers::layer(HOVERED_OUTLINE_LAYER))
                } else if selected.cue() {
                    OutlineRenderLayers(RenderLayers::layer(SELECTED_OUTLINE_LAYER))
                } else {
                    OutlineRenderLayers(RenderLayers::none())
                }
            }
            OutlineVisualization::Anchor { .. } => {
                OutlineRenderLayers(layer.cloned().unwrap_or(RenderLayers::none()))
            }
        }
    }

    pub fn depth(&self) -> OutlineMode {
        OutlineMode::ExtrudeFlat
    }

    /// If this element should use a different entity as its root for
    /// highlighting then that will be given here.
    pub fn root(&self) -> Option<Entity> {
        match self {
            OutlineVisualization::Ordinary | OutlineVisualization::Silhouette => None,
            OutlineVisualization::Anchor { body } => Some(*body),
        }
    }

    pub fn inherit_descendants(&self) -> bool {
        matches!(self, OutlineVisualization::Silhouette)
    }
}

/// Use this to temporarily prevent objects from being highlighted.
#[derive(Component)]
pub struct SuppressOutline;

pub fn add_outline_ordinary_visualization(
    mut commands: Commands,
    new_entities: Query<
        Entity,
        (
            Or<(
                Added<WallMarker>,
                Added<DoorType>,
                Added<LiftCabin<Entity>>,
                Added<MeasurementMarker>,
                Added<FiducialMarker>,
                Added<FloorMarker>,
                Added<DrawingMarker>,
                Added<ModelMarker>,
                Added<PhysicalCameraProperties>,
                Added<LightKind>,
                Added<LocationTags>,
                Added<PrimitiveShape>,
            )>,
            Without<VisualMeshMarker>,
            Without<CollisionMeshMarker>,
        ),
    >,
) {
    for e in &new_entities {
        commands
            .entity(e)
            .insert(OutlineVisualization::Ordinary)
            .insert(Selectable::new(e));
    }
}

pub fn add_outline_silhouette_visualization(
    mut commands: Commands,
    new_entities: Query<Entity, Added<ConveyorMarker>>,
) {
    for e in &new_entities {
        commands
            .entity(e)
            .insert(OutlineVisualization::Silhouette)
            .insert(Selectable::new(e));
    }
}

pub fn update_outline_visualization(
    mut commands: Commands,
    outlinable: Query<
        (
            Entity,
            &Hovered,
            &Selected,
            &OutlineVisualization,
            Option<&SuppressOutline>,
            Option<&RenderLayers>,
        ),
        Or<(
            Changed<Hovered>,
            Changed<Selected>,
            Changed<SuppressOutline>,
            Changed<ComputedVisualCue>,
        )>,
    >,
    descendants: Query<(
        Option<&Children>,
        Option<&ComputedVisualCue>,
        Option<&OutlineVisualization>,
    )>,
) {
    for (e, hovered, selected, vis, suppress, layer) in &outlinable {
        let color = if suppress.is_some() {
            None
        } else {
            vis.color(hovered, selected)
        };
        let layers = vis.layers(hovered, selected, layer);
        let depth = vis.depth();
        let root = vis.root().unwrap_or(e);

        let mut queue: SmallVec<[Entity; 10]> = SmallVec::new();
        queue.push(root);
        while let Some(top) = queue.pop() {
            if let Ok((children, cue, child_vis)) = descendants.get(top) {
                // Skip descendants with their own OutlineVisualization (and their
                // subtrees) so parent hover/selection updates do not overwrite or
                // strip a child entity's independent outline components.
                if top != root && child_vis.is_some() {
                    continue;
                }
                if let Some(cue) = cue {
                    if !cue.allow_outline {
                        // TODO(MXG): Consider if we should allow the children
                        // to be added. What if the non-outlined visual cue
                        // has descendents that should be outlined?
                        continue;
                    }
                }
                if let Some(color) = color {
                    // For Silhouette hierarchies, place OutlineVolume only on the
                    // root and InheritOutline on descendants so child meshes merge
                    // into a single outer outline.
                    if top == root || !vis.inherit_descendants() {
                        commands
                            .entity(top)
                            .insert((
                                OutlineVolume {
                                    visible: true,
                                    width: 3.0,
                                    colour: color,
                                },
                                OutlineStencil::default(),
                                ComputedOutline::default(),
                            ))
                            .insert(depth.clone())
                            .insert(layers.clone());
                    } else {
                        commands
                            .entity(top)
                            .insert((InheritOutline, ComputedOutline::default()));
                    }
                } else {
                    commands
                        .entity(top)
                        .remove::<OutlineVolume>()
                        .remove::<OutlineStencil>()
                        .remove::<ComputedOutline>()
                        .remove::<OutlineMode>()
                        .remove::<OutlineRenderLayers>()
                        .remove::<InheritOutline>();
                }

                if let Some(children) = children {
                    for child in children {
                        queue.push(*child);
                    }
                }
            }
        }
    }
}
