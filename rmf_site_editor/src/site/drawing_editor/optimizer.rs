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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::site::{
    AlignSiteDrawings, Anchor, Angle, Category, Change, ConstraintMarker,
    Distance, DrawingMarker, Edge, LevelElevation, MeasurementMarker, PixelsPerMeter,
    Pose, Rotation, SiteProperties, NameOfSite, Affiliation, Point, FiducialMarker,
};
use itertools::{Either, Itertools};
use optimization_engine::{panoc::*, *};
use std::collections::HashSet;


#[derive(SystemParam)]
pub struct OptimizationChangeParams<'w, 's> {
    pose: EventWriter<'w, 's, Change<Pose>>,
    ppm: EventWriter<'w, 's, Change<PixelsPerMeter>>,
}

#[derive(SystemParam)]
pub struct OptimizationParams<'w, 's> {
    drawings: Query<
        'w,
        's,
        (
            &'static Children,
            &'static Pose,
            &'static PixelsPerMeter,
        ),
        With<DrawingMarker>,
    >,
    global_tfs: Query<'w, 's, &'static GlobalTransform>,
    parents: Query<'w, 's, &'static Parent>,
    anchors: Query<'w, 's, &'static Anchor>,
    fiducials: Query<
        'w,
        's,
        (
            &'static Affiliation<Entity>,
            &'static Point<Entity>,
        ),
        With<FiducialMarker>,
    >,
    measurements: Query<'w, 's, &'static Edge<Entity>, With<MeasurementMarker>>,
}

pub fn align_site_drawings(
    levels: Query<(Entity, &Children, &Parent), With<LevelElevation>>,
    sites: Query<&Children, With<NameOfSite>>,
    mut events: EventReader<AlignSiteDrawings>,
    params: OptimizationParams,
    mut change: OptimizationChangeParams,
) {
    // for e in events.iter() {
    //     // Get the levels that are children of the requested site
    //     let levels = levels
    //         .iter()
    //         .filter(|(_, _, p)| ***p == **e)
    //         .collect::<Vec<_>>();
    //     let reference_level = levels
    //         .iter()
    //         .min_by(|l_a, l_b| l_a.3.elevation.partial_cmp(&l_b.3.elevation).unwrap())
    //         .expect("Site has no levels");
    //     // Reference level will be the one with minimum elevation
    //     let references = reference_level
    //         .1
    //         .iter()
    //         .filter_map(|c| {
    //             params
    //                 .drawings
    //                 .get(*c)
    //                 .ok()
    //                 .filter(|(_, _, _, _, primary)| primary.0 == true)
    //         })
    //         .map(|(e, _, _, _, _)| e)
    //         .collect::<HashSet<_>>();
    //     // Layers to be optimized are primary drawings in the non reference level
    //     let layers = levels
    //         .iter()
    //         .filter_map(|(e, c, _)| (*e != reference_level.0).then(|| c.iter()))
    //         .flatten()
    //         .filter_map(|child| params.drawings.get(*child).ok())
    //         .filter_map(|(e, _, _, _, primary)| (primary.0 == true).then(|| e))
    //         .collect::<Vec<_>>();
    //     // Inter level constraints are children of the site
    //     let constraints = sites
    //         .get(**e)
    //         .expect("Align site sent to non site entity")
    //         .iter()
    //         .filter_map(|child| params.constraints.get(*child).ok())
    //         .collect::<Vec<_>>();
    //     if constraints.is_empty() {
    //         warn!("No constraints found for site, skipping optimization");
    //         continue;
    //     }
    //     if layers.is_empty() {
    //         warn!(
    //             "No other levels drawings found for site, at least one other level must have a \
    //               primary drawing to be optimized against reference level. Skipping optimization"
    //         );
    //         continue;
    //     }
    //     if references.is_empty() {
    //         warn!(
    //             "No reference level drawing found for site. At least one primary drawing must be \
    //               present in the lowest level to use as a reference for other levels. \
    //               Skipping optimization"
    //         );
    //         continue;
    //     }
    //     for layer_entity in layers {
    //         align_drawing_pair(
    //             &references,
    //             layer_entity,
    //             &constraints,
    //             &params,
    //             &mut change,
    //         );
    //     }
    // }
}
