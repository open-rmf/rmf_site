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

use bevy::prelude::*;

use crate::site::{
    Anchor, Category, Distance, DrawingMarker, Edge, MeasurementMarker, PixelsPerMeter,
    ScaleDrawing,
};

pub fn scale_drawings(
    mut drawings: Query<(&Children, &mut PixelsPerMeter), With<DrawingMarker>>,
    measurements: Query<(&Edge<Entity>, &Distance), With<MeasurementMarker>>,
    anchors: Query<&Anchor>,
    mut events: EventReader<ScaleDrawing>,
) {
    for e in events.iter() {
        if let Ok((children, mut ppm)) = drawings.get_mut(**e) {
            let mut scale_numerator = 0.0;
            let mut scale_denominator = 0;
            for child in children {
                if let Ok((edge, distance)) = measurements.get(*child) {
                    if let Some(in_meters) = distance.0 {
                        let a0 = anchors
                            .get(edge.start())
                            .expect("Broken measurement anchor reference");
                        let d0 = a0.translation_for_category(Category::Drawing);
                        let a1 = anchors
                            .get(edge.end())
                            .expect("Broken measurement anchor reference");
                        let d1 = a1.translation_for_category(Category::Drawing);
                        let in_pixels = ((d0[0] - d1[0]) * (d0[0] - d1[0])
                            + (d0[1] - d1[1]) * (d0[1] - d1[1]))
                            .sqrt();
                        scale_numerator += in_pixels / in_meters;
                        scale_denominator += 1;
                    }
                }
            }
            if scale_denominator > 0 {
                ppm.0 = scale_numerator / (scale_denominator as f32);
            } else {
                println!("No measurements found on current drawing");
            }
        }
    }
}

/*
pub fn scale_drawing(
    level: &Level
) -> HashMap<String, Alignment> {
    let mut measurements = Vec::new();
    let mut u = Vec::new();
    let mut min_vals = Vec::new();
    let mut max_vals = Vec::new();
    let inf = std::f64::INFINITY;
    let neg_inf = std::f64::NEG_INFINITY;
    for (name, level) in &building.levels {
        let mut level_measurements = Vec::new();
        let mut initial_scale_numerator = 0.0;
        for measurement in &level.measurements {
            let p0 = level.vertices.get(measurement.0).unwrap().to_vec();
            let p1 = level.vertices.get(measurement.1).unwrap().to_vec();
            let m = Measurement {
                in_pixels: (p1 - p0).length(),
                in_meters: measurement.2.distance.1,
            };
            level_measurements.push(m);

            initial_scale_numerator += m.in_meters / m.in_pixels;
        }
        measurements.push(level_measurements);

        u.extend([
            0.0,
            0.0,
            0.0,
            initial_scale_numerator / level.measurements.len() as f64,
        ]);
        min_vals.extend([neg_inf, neg_inf, -45_f64.to_radians(), 1e-12]);
        max_vals.extend([inf, inf, 45_f64.to_radians(), 1000.0]);
    }

    let constraints = constraints::Rectangle::new(Some(&min_vals), Some(&max_vals));
    let mut panoc_cache = PANOCCache::new(u.len(), 1e-6, 10);
    let f = |u: &[f64], c: &mut f64| -> Result<(), SolverError> {
        *c = calculate_scale_cost(&measurements, u);
        Ok(())
    };

    let df = |u: &[f64], gradient: &mut [f64]| -> Result<(), SolverError> {
        calculate_scale_gradient(&measurements, u, gradient);
        Ok(())
    };
    let problem = Problem::new(&constraints, df, f);
    let mut panoc = PANOCOptimizer::new(problem, &mut panoc_cache).with_max_iter(1000);
    panoc.solve(&mut u).ok();

    names
        .into_iter()
        .zip(AllVariables::new(&u).map(|vars| vars.to_alignment()))
        .collect()
}

fn calculate_scale_cost(
    measurements: &Vec<Vec<Measurement>>,
    u: &[f64],
) -> f64 {
    let mut cost = 0.0;
    for vars_i in AllVariables::new(u) {
        if let Some(measurements_i) = measurements.get(vars_i.level) {
            for m in measurements_i {
                cost += (m.in_pixels * vars_i.scale() - m.in_meters).powi(2);
            }
        }
    }

    cost
}

fn calculate_scale_gradient(
    measurements: &Vec<Vec<Measurement>>,
    u: &[f64],
    gradient: &mut [f64],
) {
    for vars_i in AllVariables::new(u) {
        let mut grad = LevelGradient::new(vars_i.level, gradient);
        *grad.dx() = 0.0;
        *grad.dy() = 0.0;
        *grad.theta() = 0.0;
        *grad.scale() = 0.0;

        if let Some(measurements_i) = measurements.get(vars_i.level) {
            for m in measurements_i {
                *grad.scale() += 2.0 * (m.in_pixels * vars_i.scale() - m.in_meters) * m.in_pixels;
            }
        }
    }
}
*/
