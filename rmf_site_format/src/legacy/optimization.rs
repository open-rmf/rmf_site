use super::{building_map::BuildingMap, level::Alignment};
use glam::{DAffine2, DMat2, DVec2};
use optimization_engine::{panoc::*, *};
use std::{collections::HashMap, ops::RangeFrom};

pub fn align_building(building: &BuildingMap) -> HashMap<String, Alignment> {
    let mut names = Vec::new();
    let mut measurements = Vec::new();
    let mut fiducials = Vec::new();
    let mut f_map = HashMap::new();
    let mut u = Vec::new();
    let mut min_vals = Vec::new();
    let mut max_vals = Vec::new();
    let inf = std::f64::INFINITY;
    let neg_inf = std::f64::NEG_INFINITY;
    for (name, level) in &building.levels {
        names.push(name.clone());
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

        let mut level_fiducials = Vec::new();
        for fiducial in &level.fiducials {
            let f_num = f_map.len();
            let index = *f_map.entry(fiducial.2.clone()).or_insert(f_num);
            if level_fiducials.len() <= index {
                level_fiducials.resize(index + 1, None);
            }
            *level_fiducials.get_mut(index).unwrap() = Some(fiducial.to_vec());
        }
        fiducials.push(level_fiducials);

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

    {
        let f = |u: &[f64], c: &mut f64| -> Result<(), SolverError> {
            *c = calculate_scale_cost(&measurements, &fiducials, u);
            Ok(())
        };

        let df = |u: &[f64], gradient: &mut [f64]| -> Result<(), SolverError> {
            calculate_scale_gradient(&measurements, &fiducials, u, gradient);
            Ok(())
        };
        let problem = Problem::new(&constraints, df, f);
        let mut panoc = PANOCOptimizer::new(problem, &mut panoc_cache).with_max_iter(1000);
        panoc.solve(&mut u);
    }

    calculate_yaw_adjustment(&measurements, &fiducials, &mut u);
    calculate_displacement_adjustment(&measurements, &fiducials, &mut u);
    calculate_center_adjustment(building, &mut u);

    names
        .into_iter()
        .zip(AllVariables::new(&u).map(|vars| vars.to_alignment()))
        .collect()
}

struct LevelVariables<'a> {
    slice: &'a [f64],
    level: usize,
}

impl<'a> LevelVariables<'a> {
    fn new(slice: &'a [f64], level: usize) -> Self {
        Self { slice, level }
    }

    fn dx(&self) -> &f64 {
        &self.slice[0]
    }

    fn dy(&self) -> &f64 {
        &self.slice[1]
    }

    fn dp(&self) -> DVec2 {
        DVec2::new(*self.dx(), *self.dy())
    }

    fn theta(&self) -> &f64 {
        &self.slice[2]
    }

    fn rotation(&self) -> DMat2 {
        let theta = *self.theta();
        DMat2::from_angle(theta)
    }

    fn rotation_deriv(&self) -> DMat2 {
        let theta = *self.theta();
        DMat2::from_cols(
            DVec2::new(-theta.sin(), theta.cos()),
            DVec2::new(-theta.cos(), -theta.sin()),
        )
    }

    fn scale(&self) -> &f64 {
        &self.slice[3]
    }

    fn transform(&self, point: DVec2) -> DVec2 {
        let scale = *self.scale();
        scale * self.rotation() * point + self.dp()
    }

    fn to_alignment(&self) -> Alignment {
        Alignment {
            translation: self.dp(),
            rotation: *self.theta(),
            scale: *self.scale(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct AllVariables<'a> {
    slice: &'a [f64],
    next_level: usize,
}

impl<'a> AllVariables<'a> {
    fn new(slice: &'a [f64]) -> Self {
        Self {
            slice,
            next_level: 0,
        }
    }

    fn after(level: usize, slice: &'a [f64]) -> Self {
        Self {
            slice,
            next_level: level + 1,
        }
    }
}

impl<'a> Iterator for AllVariables<'a> {
    type Item = LevelVariables<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if 4 * (self.next_level + 1) <= self.slice.len() {
            let level = self.next_level;
            self.next_level += 1;
            let output = &self.slice[4 * level..4 * (level + 1)];
            Some(LevelVariables::new(output, level))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct Measurement {
    in_pixels: f64,
    in_meters: f64,
}

struct LevelGradient<'a> {
    slice: &'a mut [f64],
}

impl<'a> LevelGradient<'a> {
    fn new(level: usize, slice: &'a mut [f64]) -> Self {
        Self {
            slice: &mut slice[4 * level..4 * (level + 1)],
        }
    }

    fn dx(&mut self) -> &mut f64 {
        &mut self.slice[0]
    }

    fn dy(&mut self) -> &mut f64 {
        &mut self.slice[1]
    }

    fn theta(&mut self) -> &mut f64 {
        &mut self.slice[2]
    }

    fn scale(&mut self) -> &mut f64 {
        &mut self.slice[3]
    }
}

fn calculate_scale_cost(
    measurements: &Vec<Vec<Measurement>>,
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &[f64],
) -> f64 {
    let mut cost = 0.0;
    for vars_i in AllVariables::new(u) {
        if let Some(measurements_i) = measurements.get(vars_i.level) {
            for m in measurements_i {
                cost += (m.in_pixels * vars_i.scale() - m.in_meters).powi(2);
            }
        }

        if let Some(fiducials_i) = fiducials.get(vars_i.level) {
            for vars_j in AllVariables::after(vars_i.level, u) {
                if let Some(fiducials_j) = fiducials.get(vars_j.level) {
                    for (k, phi_ki) in fiducials_i.iter().enumerate() {
                        if let Some(phi_ki) = phi_ki {
                            if let Some(Some(phi_kj)) = fiducials_j.get(k) {
                                let f_ki = vars_i.transform(*phi_ki);
                                let f_kj = vars_j.transform(*phi_kj);
                                let delta = f_ki - f_kj;

                                for (m, phi_mi) in fiducials_i[k + 1..].iter().enumerate() {
                                    let m = m + k + 1;
                                    if let Some(phi_mi) = phi_mi {
                                        if let Some(Some(phi_mj)) = fiducials_j.get(m) {
                                            let f_mi = vars_i.transform(*phi_mi);
                                            let f_mj = vars_j.transform(*phi_mj);
                                            let df_i = f_ki - f_mi;
                                            let df_j = f_kj - f_mj;

                                            cost += (df_i.dot(df_i).sqrt() - df_j.dot(df_j).sqrt())
                                                .powi(2);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    cost
}

fn calculate_scale_gradient(
    measurements: &Vec<Vec<Measurement>>,
    fiducials: &Vec<Vec<Option<DVec2>>>,
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

        if let Some(fiducials_i) = fiducials.get(vars_i.level) {
            for vars_j in AllVariables::after(vars_i.level, u) {
                if let Some(fiducials_j) = fiducials.get(vars_j.level) {
                    for (k, phi_ki) in fiducials_i.iter().enumerate() {
                        if let Some(phi_ki) = phi_ki {
                            if let Some(Some(phi_kj)) = fiducials_j.get(k) {
                                let f_ki = vars_i.transform(*phi_ki);
                                let f_kj = vars_j.transform(*phi_kj);
                                let delta = f_ki - f_kj;

                                for (m, phi_mi) in fiducials_i[k + 1..].iter().enumerate() {
                                    let m = m + k + 1;
                                    if let Some(phi_mi) = phi_mi {
                                        if let Some(Some(phi_mj)) = fiducials_j.get(m) {
                                            let f_mi = vars_i.transform(*phi_mi);
                                            let f_mj = vars_j.transform(*phi_mj);
                                            let df_i = f_ki - f_mi;
                                            let df_j = f_kj - f_mj;

                                            let s_m_i = df_i.dot(df_i).sqrt();
                                            let s_m_j = df_j.dot(df_j).sqrt();
                                            *grad.scale() +=
                                                2.0 * (s_m_i - s_m_j) * s_m_i / vars_i.scale();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn calculate_yaw_adjustment(
    measurements: &Vec<Vec<Measurement>>,
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &mut [f64],
) {
    let mut adjustment = vec![0.0; u.len()];
    let mut weight = vec![0.0; u.len()];

    for level in 0..u.len() / 4 {
        let range = 4 * level..4 * (level + 1);
        for (v, (a, w)) in u[range.clone()].iter_mut().zip(
            adjustment[range.clone()]
                .iter()
                .zip(weight[range.clone()].iter()),
        ) {
            if *w > 0.0 {
                *v += *a / *w;
            }
        }

        let vars_i = LevelVariables::new(&u[range], level);
        if let Some(fiducials_i) = fiducials.get(vars_i.level) {
            for vars_j in AllVariables::after(vars_i.level, u) {
                let mut level_adjustment = LevelGradient::new(vars_j.level, &mut adjustment);
                let mut level_weight = LevelGradient::new(vars_j.level, &mut weight);
                if let Some(fiducials_j) = fiducials.get(vars_j.level) {
                    for (k, phi_ki) in fiducials_i.iter().enumerate() {
                        if let Some(phi_ki) = phi_ki {
                            if let Some(Some(phi_kj)) = fiducials_j.get(k) {
                                let f_ki = vars_i.transform(*phi_ki);
                                let f_kj = vars_j.transform(*phi_kj);

                                for (m, phi_mi) in fiducials_i[k + 1..].iter().enumerate() {
                                    let m = m + k + 1;
                                    if let Some(phi_mi) = phi_mi {
                                        if let Some(Some(phi_mj)) = fiducials_j.get(m) {
                                            let f_mi = vars_i.transform(*phi_mi);
                                            let f_mj = vars_j.transform(*phi_mj);
                                            let df_i = f_ki - f_mi;
                                            let df_j = f_kj - f_mj;
                                            let yaw_i = df_i.y.atan2(df_i.x);
                                            let yaw_j = df_j.y.atan2(df_j.x);

                                            *level_adjustment.theta() += yaw_i - yaw_j;
                                            *level_weight.theta() += 1.0;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn calculate_displacement_adjustment(
    measurements: &Vec<Vec<Measurement>>,
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &mut [f64],
) {
    let mut adjustment = vec![0.0; u.len()];
    let mut weight = vec![0.0; u.len()];
    for level in 0..u.len() / 4 {
        let range = 4 * level..4 * (level + 1);
        for (v, (a, w)) in u[range.clone()].iter_mut().zip(
            adjustment[range.clone()]
                .iter()
                .zip(weight[range.clone()].iter()),
        ) {
            if *w > 0.0 {
                *v += *a / *w;
            }
        }

        let vars_i = LevelVariables::new(&u[range], level);
        if let Some(fiducials_i) = fiducials.get(vars_i.level) {
            for vars_j in AllVariables::after(vars_i.level, u) {
                let mut level_adjustment = LevelGradient::new(vars_j.level, &mut adjustment);
                let mut level_weight = LevelGradient::new(vars_j.level, &mut weight);
                if let Some(fiducials_j) = fiducials.get(vars_j.level) {
                    for (k, phi_ki) in fiducials_i.iter().enumerate() {
                        if let Some(phi_ki) = phi_ki {
                            if let Some(Some(phi_kj)) = fiducials_j.get(k) {
                                let f_ki = vars_i.transform(*phi_ki);
                                let f_kj = vars_j.transform(*phi_kj);
                                let delta = f_ki - f_kj;
                                *level_adjustment.dx() += delta.x;
                                *level_weight.dx() += 1.0;

                                *level_adjustment.dy() += delta.y;
                                *level_weight.dy() += 1.0;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn calculate_center_adjustment(building: &BuildingMap, u: &mut [f64]) {
    let mut center = DVec2::ZERO;
    let mut weight = 0.0;
    for (i, level) in building.levels.values().enumerate() {
        let range = 4 * i..4 * (i + 1);
        let vars = LevelVariables::new(&u[range], i);
        for v in &level.vertices {
            let v = vars.transform(v.to_vec());
            center += v;
            weight += 1.0;
        }
    }

    if weight >= 0.0 {
        center /= weight;
        for level in 0..u.len() / 4 {
            let x = 4 * level;
            let y = 4 * level + 1;
            u[x] -= center.x;
            u[y] -= center.y;
        }
    }
}
