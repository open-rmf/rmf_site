use super::legacy::building_map::BuildingMap;
use bevy_ecs::prelude::Entity;
use glam::{DAffine2, DMat2, DVec2};
use bevy::platform::collections::HashMap;

#[derive(Default)]
pub struct SiteVariables {
    pub fiducials: Vec<FiducialVariables>,
    pub drawings: HashMap<Entity, DrawingVariables>,
}

pub struct DrawingVariables {
    pub position: DVec2,
    pub yaw: f64,
    pub scale: f64,
    pub fiducials: Vec<FiducialVariables>,
    pub measurements: Vec<MeasurementVariables>,
}

impl DrawingVariables {
    pub fn new(position: DVec2, yaw: f64, scale: f64) -> Self {
        Self {
            position,
            yaw,
            scale,
            fiducials: Default::default(),
            measurements: Default::default(),
        }
    }
}

pub struct FiducialVariables {
    pub group: Entity,
    pub position: DVec2,
}

#[derive(Clone, Copy, Debug)]
pub struct MeasurementVariables {
    pub in_pixels: f64,
    pub in_meters: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Alignment {
    pub translation: DVec2,
    pub rotation: f64,
    pub scale: f64,
}

impl Alignment {
    pub fn to_affine(&self) -> DAffine2 {
        DAffine2::from_scale_angle_translation(
            DVec2::splat(self.scale),
            self.rotation,
            self.translation,
        )
    }
}

pub fn align_site(site_variables: &SiteVariables) -> HashMap<Entity, Alignment> {
    let mut drawing_map = HashMap::new();
    let mut measurements = Vec::new();
    let mut fiducials = Vec::new();
    let mut group_map = HashMap::new();
    let mut u = Vec::new();

    let mut incorporate_fiducials = |vars: &[FiducialVariables]| {
        let mut drawing_fiducials = Vec::new();
        for FiducialVariables { group, position } in vars {
            let num_groups = group_map.len();
            let index = *group_map.entry(*group).or_insert(num_groups);
            if drawing_fiducials.len() <= index {
                drawing_fiducials.resize(index + 1, None);
            }
            *drawing_fiducials.get_mut(index).unwrap() = Some(*position);
        }
        fiducials.push(drawing_fiducials);
    };

    u.extend([0.0, 0.0, 0.0, 1.0]);
    // Measurements are irrelevant for the site
    measurements.push(Vec::new());
    incorporate_fiducials(&site_variables.fiducials);

    for (entity, drawing) in &site_variables.drawings {
        let drawing_index = measurements.len();
        measurements.push(drawing.measurements.clone());
        incorporate_fiducials(&drawing.fiducials);
        drawing_map.insert(drawing_index, *entity);
        u.extend([
            drawing.position.x,
            drawing.position.y,
            drawing.yaw,
            drawing.scale,
        ]);
    }

    solve(&mut u, &fiducials, &measurements, true);

    let mut alignments = HashMap::new();
    for (index, vars) in AllVariables::new(&u).enumerate() {
        if let Some(entity) = drawing_map.get(&index) {
            alignments.insert(*entity, vars.to_alignment());
        }
    }
    alignments
}

pub fn align_legacy_building(building: &BuildingMap) -> HashMap<String, Alignment> {
    let mut names = Vec::new();
    let mut measurements = Vec::new();
    let mut fiducials = Vec::new();
    let mut f_map = HashMap::new();
    let mut u = Vec::new();
    for (name, level) in &building.levels {
        names.push(name.clone());
        let mut level_measurements = Vec::new();
        let mut initial_scale_numerator = 0.0;
        for measurement in &level.measurements {
            let p0 = level.vertices.get(measurement.0).unwrap().to_vec();
            let p1 = level.vertices.get(measurement.1).unwrap().to_vec();
            let m = MeasurementVariables {
                in_pixels: (p1 - p0).length(),
                in_meters: measurement.2.distance.1,
            };
            level_measurements.push(m);

            initial_scale_numerator += m.in_meters / m.in_pixels;
        }
        // If no measurements are available, default to a standard 5cm per pixel
        let initial_scale = if !level.measurements.is_empty() {
            initial_scale_numerator / level.measurements.len() as f64
        } else {
            0.05
        };
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

        u.extend([0.0, 0.0, 0.0, initial_scale]);
    }

    solve(&mut u, &fiducials, &measurements, false);
    calculate_center_adjustment(building, &mut u);

    names
        .into_iter()
        .zip(AllVariables::new(&u).map(|vars| vars.to_alignment()))
        .collect()
}

fn solve(
    u: &mut [f64],
    fiducials: &Vec<Vec<Option<DVec2>>>,
    measurements: &Vec<Vec<MeasurementVariables>>,
    has_ground_truth: bool,
) {
    let df_scale = |u: &[f64], gradient: &mut [f64]| {
        calculate_scale_gradient(has_ground_truth, &measurements, &fiducials, u, gradient);
        if has_ground_truth {
            for i in 0..4 {
                gradient[i] = 0.0;
            }
        }
    };
    gradient_descent(1e-6, 1.0, 100, u, df_scale);

    let df_yaw = |u: &[f64], gradient: &mut [f64]| {
        calculate_yaw_gradient(has_ground_truth, &fiducials, u, gradient);
    };
    gradient_descent(1e-6, 1.0, 100, u, df_yaw);

    let df_displacement = |u: &[f64], gradient: &mut [f64]| {
        calculate_displacement_gradient(has_ground_truth, &fiducials, u, gradient);
    };
    gradient_descent(1e-6, 1.0, 100, u, df_displacement);
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
    except: Option<usize>,
}

impl<'a> AllVariables<'a> {
    fn new(slice: &'a [f64]) -> Self {
        Self {
            slice,
            next_level: 0,
            except: None,
        }
    }

    fn after(level: usize, slice: &'a [f64]) -> Self {
        Self {
            slice,
            next_level: level + 1,
            except: None,
        }
    }

    fn except(level: usize, slice: &'a [f64]) -> Self {
        Self {
            slice,
            next_level: 0,
            except: Some(level),
        }
    }
}

impl<'a> Iterator for AllVariables<'a> {
    type Item = LevelVariables<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.except.is_some_and(|except| self.next_level == except) {
            self.next_level += 1;
        }

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

fn gradient_descent(
    gradient_threshold: f64,
    gamma: f64,
    iteration_limit: usize,
    u: &mut [f64],
    df: impl Fn(&[f64], &mut [f64]),
) {
    let mut iterations = 0;
    let mut gradient = Vec::new();
    gradient.resize(u.len(), 0.0);
    loop {
        df(u, &mut gradient);
        let mut grad_length = 0.0;
        for (x, dx) in u.iter_mut().zip(gradient.iter()) {
            *x = *x - gamma * *dx;
            grad_length += *dx * *dx;
        }
        if grad_length.sqrt() <= gradient_threshold {
            break;
        }

        iterations += 1;
        if iterations >= iteration_limit {
            break;
        }
    }
}

fn calculate_scale_gradient(
    has_ground_truth: bool,
    measurements: &Vec<Vec<MeasurementVariables>>,
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &[f64],
    gradient: &mut [f64],
) {
    for x in gradient.iter_mut() {
        *x = 0.0;
    }

    let mut weight = 0.0;

    for vars_i in AllVariables::new(u) {
        let mut grad = LevelGradient::new(vars_i.level, gradient);
        if let Some(measurements_i) = measurements.get(vars_i.level) {
            for m in measurements_i {
                *grad.scale() += vars_i.scale() - m.in_meters / m.in_pixels;
                weight += 1.0;
            }
        }
    }

    for vars_i in AllVariables::new(u) {
        let Some(fiducials_i) = fiducials.get(vars_i.level) else {
            continue;
        };
        for vars_j in AllVariables::except(vars_i.level, u) {
            let Some(fiducials_j) = fiducials.get(vars_j.level) else {
                continue;
            };
            for (k, phi_ki) in fiducials_i.iter().enumerate() {
                let Some(phi_ki) = phi_ki else { continue };
                let Some(Some(phi_kj)) = fiducials_j.get(k) else {
                    continue;
                };
                let f_ki = vars_i.transform(*phi_ki);
                let f_kj = vars_j.transform(*phi_kj);

                for (m, phi_mi) in fiducials_i[k + 1..].iter().enumerate() {
                    let m = m + k + 1;
                    let Some(phi_mi) = phi_mi else { continue };
                    let Some(Some(phi_mj)) = fiducials_j.get(m) else {
                        continue;
                    };
                    let f_mi = vars_i.transform(*phi_mi);
                    let f_mj = vars_j.transform(*phi_mj);
                    let df_i = f_ki - f_mi;
                    let df_j = f_kj - f_mj;

                    let s_m_i = df_i.dot(df_i).sqrt();
                    let s_m_j = df_j.dot(df_j).sqrt();

                    if !(has_ground_truth && vars_i.level == 0) {
                        let mut grad_i = LevelGradient::new(vars_i.level, gradient);
                        *grad_i.scale() += vars_i.scale() * (1.0 - s_m_j / s_m_i);
                        weight += 1.0;
                    }

                    let mut grad_j = LevelGradient::new(vars_j.level, gradient);
                    *grad_j.scale() += vars_j.scale() * (1.0 - s_m_i / s_m_j);
                    weight += 1.0;
                }
            }
        }
    }

    for x in gradient.iter_mut() {
        *x /= f64::max(weight, 1.0);
    }
}

fn calculate_yaw_gradient(
    has_ground_truth: bool,
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &[f64],
    gradient: &mut [f64],
) {
    for x in gradient.iter_mut() {
        *x = 0.0;
    }

    let mut weight = 0.0;

    traverse_yaws(fiducials, u, |i, yaw_i, j, yaw_j| {
        if !(has_ground_truth && i == 0) {
            *LevelGradient::new(i, gradient).theta() += yaw_i - yaw_j;
            weight += 1.0;
        }

        *LevelGradient::new(j, gradient).theta() += yaw_j - yaw_i;
        weight += 1.0;
    });

    for x in gradient.iter_mut() {
        *x /= f64::max(weight, 1.0);
    }
}

fn traverse_yaws<F: FnMut(usize, f64, usize, f64)>(
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &[f64],
    mut f: F,
) {
    for vars_i in AllVariables::new(u) {
        let Some(fiducials_i) = fiducials.get(vars_i.level) else {
            continue;
        };
        for vars_j in AllVariables::after(vars_i.level, u) {
            let Some(fiducials_j) = fiducials.get(vars_j.level) else {
                continue;
            };
            for (k, phi_ki) in fiducials_i.iter().enumerate() {
                let Some(phi_ki) = phi_ki else { continue };
                let Some(Some(phi_kj)) = fiducials_j.get(k) else {
                    continue;
                };
                let f_ki = vars_i.transform(*phi_ki);
                let f_kj = vars_j.transform(*phi_kj);

                for (m, phi_mi) in fiducials_i[k + 1..].iter().enumerate() {
                    let m = m + k + 1;
                    let Some(phi_mi) = phi_mi else { continue };
                    let Some(Some(phi_mj)) = fiducials_j.get(m) else {
                        continue;
                    };
                    let f_mi = vars_i.transform(*phi_mi);
                    let f_mj = vars_j.transform(*phi_mj);
                    let df_i = f_ki - f_mi;
                    let df_j = f_kj - f_mj;
                    let yaw_i = f64::atan2(df_i.y, df_i.x);
                    let yaw_j = f64::atan2(df_j.y, df_j.x);

                    f(vars_i.level, yaw_i, vars_j.level, yaw_j);
                }
            }
        }
    }
}

fn calculate_displacement_gradient(
    has_ground_truth: bool,
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &[f64],
    gradient: &mut [f64],
) {
    for x in gradient.iter_mut() {
        *x = 0.0;
    }

    let mut weight = 0.0;

    traverse_locations(fiducials, u, |i, f_ki, j, f_kj| {
        let delta = f_ki - f_kj;
        if !(has_ground_truth && i == 0) {
            let mut grad_i = LevelGradient::new(i, gradient);
            *grad_i.dx() += delta.x;
            *grad_i.dy() += delta.y;
            weight += 1.0;
        }

        let mut grad_j = LevelGradient::new(j, gradient);
        *grad_j.dx() += -delta.x;
        *grad_j.dy() += -delta.y;
        weight += 1.0;
    });

    for x in gradient.iter_mut() {
        *x /= f64::max(weight, 1.0);
    }
}

fn traverse_locations<F: FnMut(usize, DVec2, usize, DVec2)>(
    fiducials: &Vec<Vec<Option<DVec2>>>,
    u: &[f64],
    mut f: F,
) {
    for vars_i in AllVariables::new(u) {
        let Some(fiducials_i) = fiducials.get(vars_i.level) else {
            continue;
        };
        for vars_j in AllVariables::after(vars_i.level, u) {
            let Some(fiducials_j) = fiducials.get(vars_j.level) else {
                continue;
            };
            for (k, phi_ki) in fiducials_i.iter().enumerate() {
                let Some(phi_ki) = phi_ki else { continue };
                let Some(Some(phi_kj)) = fiducials_j.get(k) else {
                    continue;
                };
                let f_ki = vars_i.transform(*phi_ki);
                let f_kj = vars_j.transform(*phi_kj);
                f(vars_i.level, f_ki, vars_j.level, f_kj);
            }
        }
    }
}

fn calculate_center_adjustment(building: &BuildingMap, u: &mut [f64]) {
    if building.levels.is_empty() {
        return;
    }
    let reference_idx = building
        .reference_level_name
        .as_ref()
        .and_then(|name| {
            building
                .levels
                .iter()
                .position(|(level_name, _)| name == level_name)
        })
        .unwrap_or(0);
    let range = 4 * reference_idx..4 * (reference_idx + 1);
    let center = LevelVariables::new(&u[range], reference_idx);
    let dx = center.dx().clone();
    let dy = center.dy().clone();

    for level in 0..u.len() / 4 {
        let x = 4 * level;
        let y = 4 * level + 1;
        u[x] -= dx;
        u[y] -= dy;
    }
}
