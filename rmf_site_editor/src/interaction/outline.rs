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

use crate::interaction::*;
use bevy::render::view::RenderLayers;
use bevy_mod_outline::{OutlineBundle, OutlineRenderLayers, OutlineVolume, SetOutlineDepth};
use rmf_site_format::{
    DoorType, FloorMarker, LiftCabin, LightKind, LocationTags, MeasurementMarker, ModelMarker,
    PhysicalCameraProperties, WallMarker,
};
use smallvec::SmallVec;

// TODO(MXG): Customize the behavior of floor, wall, and model visual cues.
// For now we just use the same interaction behavior for all of them.
#[derive(Component)]
pub enum OutlineVisualization {
    Ordinary,
    Anchor,
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
            OutlineVisualization::Anchor => {
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
            OutlineVisualization::Anchor => {
                OutlineRenderLayers(RenderLayers::layer(XRAY_RENDER_LAYER))
            }
        }
    }

    // A strange issue is causing outlines to diverge at certain camera angles
    // for objects that use the Flat depth. The issue doesn't seem to happen
    // for objects with Real depth, so we are switching most objects to use the
    // Real outline setting. However, I don't think this looks good for certain
    // types of objects so we will keep the Flat setting for them and accept
    // that certain camera angles can make their outline look strange.
    //
    // The relevant upstream issue is being tracked here: https://github.com/komadori/bevy_mod_outline/issues/14
    pub fn depth(&self) -> SetOutlineDepth {
        SetOutlineDepth::Flat {
            model_origin: Vec3::ZERO,
        }
    }
}

pub fn add_outline_visualization(
    mut commands: Commands,
    new_entities: Query<
        Entity,
        Or<(
            Added<WallMarker>,
            Added<DoorType>,
            Added<LiftCabin<Entity>>,
            Added<MeasurementMarker>,
            Added<FloorMarker>,
            Added<ModelMarker>,
            Added<PhysicalCameraProperties>,
            Added<LightKind>,
            Added<LocationTags>,
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
        (Entity, &Hovered, &Selected, &OutlineVisualization),
        Or<(Changed<Hovered>, Changed<Selected>)>,
    >,
    descendants: Query<(Option<&Children>, Option<&ComputedVisualCue>)>,
) {
    for (e, hovered, selected, vis) in &outlinable {
        let color = vis.color(hovered, selected);
        let layers = vis.layers(hovered, selected);
        let depth = vis.depth();

        let mut queue: SmallVec<[Entity; 10]> = SmallVec::new();
        queue.push(e);
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
                        .remove::<SetOutlineDepth>();
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
