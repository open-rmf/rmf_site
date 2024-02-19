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
use bevy::{
    math::{DVec2, Vec2},
    prelude::*,
};

use crate::site::{
    Affiliation, AlignSiteDrawings, Anchor, Angle, Category, Distance, DrawingMarker, Edge,
    FiducialMarker, LevelElevation, MeasurementMarker, NameOfSite, PixelsPerMeter, Point, Pose,
    Rotation,
};

use rmf_site_format::alignment::{
    align_site, DrawingVariables, FiducialVariables, MeasurementVariables, SiteVariables,
};

#[derive(SystemParam)]
pub struct OptimizationParams<'w, 's> {
    drawings: Query<
        'w,
        's,
        (
            &'static Children,
            &'static mut Pose,
            &'static mut PixelsPerMeter,
        ),
        With<DrawingMarker>,
    >,
    anchors: Query<'w, 's, &'static Anchor>,
    fiducials:
        Query<'w, 's, (&'static Affiliation<Entity>, &'static Point<Entity>), With<FiducialMarker>>,
    measurements:
        Query<'w, 's, (&'static Edge<Entity>, &'static Distance), With<MeasurementMarker>>,
}

pub fn align_site_drawings(
    levels: Query<&Children, With<LevelElevation>>,
    sites: Query<&Children, With<NameOfSite>>,
    mut events: EventReader<AlignSiteDrawings>,
    mut params: OptimizationParams,
) {
    for AlignSiteDrawings(site) in events.read() {
        let mut site_variables = SiteVariables::<Entity>::default();
        let Ok(children) = sites.get(*site) else {
            continue;
        };
        for child in children {
            let Ok((group, point)) = params.fiducials.get(*child) else {
                continue;
            };
            let Ok(anchor) = params.anchors.get(point.0) else {
                continue;
            };
            let Some(group) = group.0 else { continue };
            let p = anchor.translation_for_category(Category::Fiducial);
            site_variables.fiducials.push(FiducialVariables {
                group,
                position: DVec2::new(p[0] as f64, p[1] as f64),
            });
        }

        for child in children {
            let Ok(level_children) = levels.get(*child) else {
                continue;
            };
            for level_child in level_children {
                let Ok((drawing_children, pose, ppm)) = params.drawings.get(*level_child) else {
                    continue;
                };
                let mut drawing_variables = DrawingVariables::<Entity>::new(
                    Vec2::from_slice(&pose.trans).as_dvec2(),
                    pose.rot.yaw().radians() as f64,
                    (1.0 / ppm.0) as f64,
                );
                for child in drawing_children {
                    if let Ok((group, point)) = params.fiducials.get(*child) {
                        let Ok(anchor) = params.anchors.get(point.0) else {
                            continue;
                        };
                        let Some(group) = group.0 else { continue };
                        let p = anchor.translation_for_category(Category::Fiducial);
                        drawing_variables.fiducials.push(FiducialVariables {
                            group,
                            position: DVec2::new(p[0] as f64, p[1] as f64),
                        });
                    }

                    if let Ok((edge, distance)) = params.measurements.get(*child) {
                        let Ok([anchor0, anchor1]) = params.anchors.get_many(edge.array()) else {
                            continue;
                        };
                        let Some(in_meters) = distance.0 else {
                            continue;
                        };
                        let in_meters = in_meters as f64;
                        let p0 =
                            Vec2::from_slice(anchor0.translation_for_category(Category::Fiducial));
                        let p1 =
                            Vec2::from_slice(anchor1.translation_for_category(Category::Fiducial));
                        let in_pixels = (p1 - p0).length() as f64;
                        drawing_variables.measurements.push(MeasurementVariables {
                            in_pixels,
                            in_meters,
                        });
                    }
                }

                site_variables
                    .drawings
                    .insert(*level_child, drawing_variables);
            }
        }

        // TODO(@mxgrey): When we implement an undo buffer, remember to make an
        // undo operation for this set of changes.
        let alignments = align_site(&site_variables);
        for (e, alignment) in alignments {
            let Ok((_, mut pose, mut ppm)) = params.drawings.get_mut(e) else {
                continue;
            };
            pose.trans[0] = alignment.translation.x as f32;
            pose.trans[1] = alignment.translation.y as f32;
            pose.rot =
                Rotation::Yaw(Angle::Rad(alignment.rotation as f32).match_variant(pose.rot.yaw()));
            ppm.0 = 1.0 / alignment.scale as f32;
        }
    }
}
