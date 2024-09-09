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

use crate::{interaction::*, site::DrawingMarker};
use bevy::render::view::RenderLayers;
use bevy_mod_outline::{OutlineBundle, OutlineMode, OutlineRenderLayers, OutlineVolume};
use rmf_site_format::{
    DoorType, FiducialMarker, FloorMarker, LiftCabin, LightKind, LocationTags, MeasurementMarker,
    ModelMarker, PhysicalCameraProperties, PrimitiveShape, WallMarker,
};
use smallvec::SmallVec;

// TODO(MXG): Customize the behavior of floor, wall, and model visual cues.
// For now we just use the same interaction behavior for all of them.
#[derive(Component)]
pub enum OutlineVisualization {
    Ordinary,
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
            OutlineVisualization::Ordinary => {
                if !hovered.cue() && !selected.cue() {
                    None
                } else if hovered.cue() && selected.cue() {
                    Some(Color::rgb(1.0, 0.0, 0.3))
                } else if selected.cue() {
                    Some(Color::rgb(1.0, 0.3, 1.0))
                } else
                /* only hovered */
                {
                    Some(Color::WHITE)
                }
            }
            OutlineVisualization::Anchor { .. } => {
                if hovered.is_hovered {
                    Some(Color::WHITE)
                } else {
                    Some(Color::BLACK)
                }
            }
        }
    }

    pub fn layers(&self, hovered: &Hovered, selected: &Selected) -> OutlineRenderLayers {
        match self {
            OutlineVisualization::Ordinary => {
                if hovered.cue() {
                    OutlineRenderLayers(RenderLayers::layer(HOVERED_OUTLINE_LAYER))
                } else if selected.cue() {
                    OutlineRenderLayers(RenderLayers::layer(SELECTED_OUTLINE_LAYER))
                } else {
                    OutlineRenderLayers(RenderLayers::none())
                }
            }
            OutlineVisualization::Anchor { .. } => {
                OutlineRenderLayers(RenderLayers::layer(XRAY_RENDER_LAYER))
            }
        }
    }

    pub fn depth(&self) -> OutlineMode {
        OutlineMode::FlatVertex {
            model_origin: Vec3::ZERO,
        }
    }

    /// If this element should use a different entity as its root for
    /// highlighting then that will be given here.
    pub fn root(&self) -> Option<Entity> {
        match self {
            OutlineVisualization::Ordinary => None,
            OutlineVisualization::Anchor { body } => Some(*body),
        }
    }
}

/// Use this to temporarily prevent objects from being highlighted.
#[derive(Component)]
pub struct SuppressOutline;

pub fn add_outline_visualization(
    mut commands: Commands,
    new_entities: Query<
        Entity,
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
    >,
) {
    for e in &new_entities {
        commands
            .entity(e)
            .insert(OutlineVisualization::default())
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
        ),
        Or<(
            Changed<Hovered>,
            Changed<Selected>,
            Changed<SuppressOutline>,
        )>,
    >,
    descendants: Query<(Option<&Children>, Option<&ComputedVisualCue>)>,
) {
    for (e, hovered, selected, vis, suppress) in &outlinable {
        let color = if suppress.is_some() {
            None
        } else {
            vis.color(hovered, selected)
        };
        let layers = vis.layers(hovered, selected);
        let depth = vis.depth();
        let root = vis.root().unwrap_or(e);

        let mut queue: SmallVec<[Entity; 10]> = SmallVec::new();
        queue.push(root);
        while let Some(top) = queue.pop() {
            if let Ok((children, cue)) = descendants.get(top) {
                if let Some(cue) = cue {
                    if !cue.allow_outline {
                        // TODO(MXG): Consider if we should allow the children
                        // to be added. What if the non-outlined visual cue
                        // has descendents that should be outlined?
                        continue;
                    }
                }
                if let Some(color) = color {
                    commands
                        .entity(top)
                        .insert(OutlineBundle {
                            outline: OutlineVolume {
                                visible: true,
                                width: 3.0,
                                colour: color,
                            },
                            ..default()
                        })
                        .insert(depth.clone())
                        .insert(layers);
                } else {
                    commands
                        .entity(top)
                        .remove::<OutlineBundle>()
                        .remove::<OutlineMode>();
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
