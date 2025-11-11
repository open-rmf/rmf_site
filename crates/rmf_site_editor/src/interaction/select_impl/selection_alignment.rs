/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use bevy::{ecs::relationship::AncestorIter, prelude::*};
use crossflow::prelude::{
    BufferAccess, BufferKey, ContinuousQuery, ContinuousService, ContinuousServiceInput, Service,
};

use super::CreationSettings;

use crate::{
    interaction::{Cursor, Hovered, IntersectGroundPlaneParams, Preview},
    site::{Anchor, Category, CurrentLevel, Edge, LiftCabin, Pending},
};

/// The alignment system needs some context about what is being created so it
/// can make appropriate calculations for how to align the cursor. Element
/// creation workflows will pass this struct into a buffer that is read by the
/// alignment system.
pub struct SelectionAlignmentBasis {
    /// When creating an edge, this will contain the ID of the selected "start"
    /// anchor for the edge. If no "start" anchor has been selected yet, this
    /// will be [`None`].
    ///
    /// When creating a path, this will contain the last selected anchor of the
    /// path. If no anchor has been selected yet, this will be [`None`].
    ///
    /// When creating a point (or model), this will always be [`None`].
    ///
    /// In all cases, this represents the last selected anchor which forms a
    /// straight line with the next anchor that is to be selected. The alignment
    /// system will try to position the next anchor in a way that aligns this
    /// line with an appopriate reference line.
    ///
    /// When this is [`None`], the next selected anchor will be snapped to a
    /// nearby straight line.
    pub base_anchor: Option<Entity>,
}

impl SelectionAlignmentBasis {
    /// Set a new base anchor for alignment
    pub fn new(base_anchor: Entity) -> Self {
        Self {
            base_anchor: Some(base_anchor),
        }
    }

    /// Set the basis to none, which means we are aligning a single anchor, not
    /// an edge or a path segment.
    pub fn none() -> Self {
        Self { base_anchor: None }
    }
}

pub type CursorTransformService = Service<BufferKey<SelectionAlignmentBasis>, ()>;

/// Update the virtual cursor transform while in select anchor mode
pub fn select_anchor_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<BufferKey<SelectionAlignmentBasis>, ()>,
    orders: ContinuousQuery<BufferKey<SelectionAlignmentBasis>, ()>,
    basis: BufferAccess<SelectionAlignmentBasis>,
    cursor: Res<Cursor>,
    mut transforms: Query<&mut Transform>,
    intersect_ground_params: IntersectGroundPlaneParams,
    settings: Res<CreationSettings>,
    edges: Query<(Entity, &Edge<Entity>), (Without<Preview>, Without<Pending>)>,
    anchors: Query<&Anchor>,
    parents: Query<&ChildOf>,
    lifts: Query<(), With<LiftCabin<Entity>>>,
    current_level: Res<CurrentLevel>,
    mut hovered: Query<&mut Hovered>,
    mut support_hovered: Local<Option<Entity>>,
    mut cache: Local<Option<AlignmentCache>>,
) {
    let Some(orders) = orders.view(&key) else {
        if let Some(previous) = support_hovered.take() {
            if let Ok(mut hovered) = hovered.get_mut(previous) {
                hovered.support_hovering.remove(&key.provider());
            }
        }

        return;
    };

    let Some(order) = orders.iter().next() else {
        *cache = None;
        if let Some(previous) = support_hovered.take() {
            if let Ok(mut hovered) = hovered.get_mut(previous) {
                hovered.support_hovering.remove(&key.provider());
            }
        }

        return;
    };

    let intersection = match intersect_ground_params.ground_plane_intersection() {
        Some(intersection) => intersection,
        None => {
            return;
        }
    };

    let x = intersection.translation.truncate();
    let position = if settings.alignment_on {
        if let Some(level) = **current_level {
            let base_anchor_id = basis
                .get_newest(order.request())
                .map(|s| s.base_anchor)
                .flatten();

            let base_anchor = base_anchor_id.map(|e| anchors.get(e).ok()).flatten();

            // TODO(@mxgrey): Test whether the caching makes a significant
            // difference on performance at various site sizes. Also consider
            // whether there may be more effective caching methods.
            if cache.as_ref().is_some_and(|c| c.last_order != order.id()) {
                // Reset the cache because we're handling a new order, so there could
                // be new items to align with.
                *cache = None;
            }

            if cache
                .as_ref()
                .is_some_and(|c| c.last_base_anchor != base_anchor_id)
            {
                *cache = None;
            }

            if cache.as_ref().is_some_and(|c| {
                (c.center - x).length_squared() > settings.alignment_cache_deadzone
            }) {
                *cache = None;
            }

            let site_id = parents.get(level).ok().map(|c| c.parent());
            let is_site_anchor = |e| {
                if let Some(site_id) = site_id {
                    if let Ok(child_of) = parents.get(e) {
                        return child_of.parent() == site_id;
                    }
                }
                return false;
            };

            let is_on_level = |e| AncestorIter::new(&parents, e).any(|p| p == level);

            let is_lift_anchor = |e| AncestorIter::new(&parents, e).any(|p| lifts.contains(p));

            // If the cache is not set, create it now
            let cache = cache.get_or_insert_with(|| {
                let mut lines = Vec::new();
                for (edge_id, edge) in &edges {
                    let mut on_level = true;
                    for e in edge.array() {
                        on_level &= is_site_anchor(e) || is_on_level(e) || is_lift_anchor(e);
                    }

                    if on_level {
                        if let (Ok(a0), Ok(a1)) =
                            (anchors.get(edge.start()), anchors.get(edge.end()))
                        {
                            let p0: Vec2 = a0.translation_for_category(Category::General).into();
                            let p1: Vec2 = a1.translation_for_category(Category::General).into();
                            if settings
                                .alignment_window
                                .is_none_or(|d| distance_to_line_segment(x, p0, p1) < d)
                            {
                                if let Some(dir) = (p1 - p0).try_normalize() {
                                    lines.push(Line {
                                        element: Some(edge_id),
                                        p0,
                                        dir,
                                    });
                                }
                            }
                        }
                    }
                }

                if let Some(base_anchor) = base_anchor {
                    // Always include the plain x and y axis directions if we
                    // have a base anchor.
                    let p0: Vec2 = base_anchor
                        .translation_for_category(Category::General)
                        .into();
                    lines.push(Line::x_axis(p0));
                    lines.push(Line::y_axis(p0));
                }

                AlignmentCache {
                    last_order: order.id(),
                    last_base_anchor: base_anchor_id,
                    center: x,
                    lines,
                }
            });

            // Note: If the alignment basis has never been set then we assume "none",
            // meaning we are aligning a single point.
            let choice = if let Some(base_anchor) = base_anchor {
                // We are aligning an edge
                align_edge(x, base_anchor, &cache.lines)
            } else {
                // We are aligning a single point
                align_point(x, &cache.lines)
            };

            if *support_hovered != choice.element {
                if let Some(previous) = support_hovered.take() {
                    if let Ok(mut hovered) = hovered.get_mut(previous) {
                        hovered.support_hovering.remove(&key.provider());
                    }
                }

                if let Some(chosen) = choice.element {
                    if let Ok(mut hovered) = hovered.get_mut(chosen) {
                        hovered.support_hovering.insert(key.provider());
                        *support_hovered = Some(chosen);
                    }
                }
            }

            choice.x_prime
        } else {
            x
        }
    } else {
        *cache = None;
        if let Some(previous) = support_hovered.take() {
            if let Ok(mut hovered) = hovered.get_mut(previous) {
                hovered.support_hovering.remove(&key.provider());
            }
        }

        x
    };

    let mut transform = match transforms.get_mut(cursor.frame) {
        Ok(transform) => transform,
        Err(_) => {
            return;
        }
    };

    *transform = Transform::from_translation(position.extend(0.0));
}

pub struct AlignmentCache {
    last_order: Entity,
    last_base_anchor: Option<Entity>,
    center: Vec2,
    lines: Vec<Line>,
}

pub struct Line {
    element: Option<Entity>,
    p0: Vec2,
    dir: Vec2,
}

impl Line {
    fn x_axis(p0: Vec2) -> Self {
        Self {
            element: None,
            p0,
            dir: Vec2::X,
        }
    }

    fn y_axis(p0: Vec2) -> Self {
        Self {
            element: None,
            p0,
            dir: Vec2::Y,
        }
    }
}

#[derive(Clone, Copy)]
struct LineChoice {
    cost: f32,
    x_prime: Vec2,
    element: Option<Entity>,
}

impl LineChoice {
    fn fallback(x_prime: Vec2) -> Self {
        Self {
            x_prime,
            cost: 0.0,
            element: None,
        }
    }

    fn evaluate(self, best: &mut Option<Self>) {
        if let Some(former_best) = best {
            if self.cost < former_best.cost {
                *best = Some(self);
            }
        } else {
            *best = Some(self);
        }
    }
}

fn align_edge(x: Vec2, base_anchor: &Anchor, lines: &Vec<Line>) -> LineChoice {
    let p0: Vec2 = base_anchor
        .translation_for_category(Category::General)
        .into();

    let mut best: Option<LineChoice> = None;
    for line in lines {
        let x_prime = p0 + (x - p0).dot(line.dir) * line.dir;
        let cost = (x - x_prime).length_squared();
        let element = line.element;
        let choice = LineChoice {
            cost,
            x_prime,
            element,
        };
        choice.evaluate(&mut best);
    }

    best.unwrap_or(LineChoice::fallback(x))
}

fn align_point(x: Vec2, lines: &Vec<Line>) -> LineChoice {
    let mut best: Option<LineChoice> = None;
    for line in lines {
        let x_prime = line.p0 + (x - line.p0).dot(line.dir) * line.dir;
        let cost = (x - x_prime).length_squared();
        let element = line.element;
        let choice = LineChoice {
            cost,
            x_prime,
            element,
        };
        choice.evaluate(&mut best);
    }

    best.unwrap_or(LineChoice::fallback(x))
}

fn distance_to_line_segment(x: Vec2, p0: Vec2, p1: Vec2) -> f32 {
    let Some(dir) = (p1 - p0).try_normalize() else {
        // This will get filtered out later
        return 0.0;
    };

    let s = (x - p0).dot(dir);
    if s < 0.0 {
        // The point is behind the start point of the line segment
        return (x - p0).length();
    } else if (p1 - p0).length() < s {
        // The point is beyond the end point of the line segment
        return (x - p1).length();
    } else {
        // The point is in between both endpoints of the line segment
        let x_proj = p0 + s * dir;
        return (x - x_proj).length();
    }
}
