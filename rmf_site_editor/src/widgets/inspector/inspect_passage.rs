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

use bevy_egui::egui::{Ui, DragValue};
use rmf_site_format::*;

pub struct InspectPassage<'a> {
    pub params: &'a PassageCells,
    pub coords: Option<[i32; 2]>,
}

impl<'a> InspectPassage<'a> {
    pub fn new(cells: &'a PassageCells, coords: Option<[i32; 2]>) -> Self {
        Self { params: cells, coords }
    }

    pub fn show(self, ui: &mut Ui) -> Option<PassageCells> {
        let mut new_lanes = self.params.lanes;
        ui.horizontal(|ui| {
            ui.label("Lanes");
            ui.add(
                DragValue::new(&mut new_lanes)
                .fixed_decimals(0)
                .speed(0.1)
                .clamp_range(1.0..=f64::INFINITY)
            );
        });

        let mut new_cell_size = self.params.cell_size;
        ui.horizontal(|ui| {
            ui.label("Cell Size");
            ui.add(
                DragValue::new(&mut new_cell_size)
                .max_decimals(2)
                .clamp_range(0.1..=3.0)
                .speed(0.01)
            );
        });

        let mut new_start_overflow = self.params.overflow[0];
        let mut new_end_overflow = self.params.overflow[1];
        ui.horizontal(|ui| {
            ui.label("Overflow");
            ui.add(
                DragValue::new(&mut new_start_overflow)
                .fixed_decimals(0)
                .speed(0.1)
            ).on_hover_text("How many rows should overflow past the passage's start anchor");
            ui.add(
                DragValue::new(&mut new_end_overflow)
                .fixed_decimals(0)
                .speed(0.1)
            ).on_hover_text("How many rows should overflow past the passage's end anchor");
        });

        ui.heading("Default constraints");
        let new_default_constraints = InspectCellConstraints::new(&self.params.default_constraints).show(ui);

        let new_cell_constraint = if let Some(coords) = self.coords {
            ui.heading("Cell constraints");
            let custom = self.params.constraints.get(&coords);
            let mut use_default = custom.is_none();
            ui.checkbox(&mut use_default, "Use default");
            if let Some(custom) = custom {
                if use_default {
                    // The user has chosen to use the default constraints, but
                    // there is currently a custom entry for this cell. Therefore
                    // we must remove it.
                    Some((coords, None))
                } else {
                    InspectCellConstraints::new(custom).show(ui).map(
                        |c| (coords, Some(c))
                    )
                }
            } else if !use_default {
                // The user has chosen to not use the default constraints for
                // this cell, but there is not currently a custom constraint for
                // the cell so we need to add one.
                Some((coords, Some(self.params.default_constraints.clone())))
            } else {
                None
            }
        } else {
            None
        };

        if new_lanes != self.params.lanes || new_cell_size != self.params.cell_size
            || new_start_overflow != self.params.overflow[0]
            || new_end_overflow != self.params.overflow[1]
            || new_default_constraints.is_some()
            || new_cell_constraint.is_some()
        {
            let mut new_cells = self.params.clone();
            new_cells.lanes = new_lanes;
            new_cells.cell_size = new_cell_size;
            new_cells.overflow = [new_start_overflow, new_end_overflow];

            if let Some(new_default_constraints) = new_default_constraints {
                new_cells.default_constraints = new_default_constraints;
            }

            if let Some((coords, constraint)) = new_cell_constraint {
                if let Some(constraint) = constraint {
                    new_cells.constraints.insert(coords, constraint);
                } else {
                    new_cells.constraints.remove(&coords);
                }
            }

            return Some(new_cells);
        }

        None
    }
}

struct InspectCellConstraints<'a> {
    constraints: &'a CellConstraints,
}

impl<'a> InspectCellConstraints<'a> {
    fn new(constraints: &'a CellConstraints) -> Self {
        Self { constraints }
    }

    fn show(self, ui: &mut Ui) -> Option<CellConstraints> {
        let mut new_constraints = self.constraints.clone();
        for (label, direction) in [
            ("Forward", &mut new_constraints.forward),
            ("Backward", &mut new_constraints.backward),
            ("Left", &mut new_constraints.left),
            ("Right", &mut new_constraints.right),
        ] {
            let mut unconstrained = direction.is_unconstrained();
            ui.checkbox(&mut unconstrained, label);
            if unconstrained {
                *direction = CellTransition::Unconstrained;
            } else {
                *direction = CellTransition::Disabled;
            }
        }

        if new_constraints != *self.constraints {
            return Some(new_constraints);
        }

        None
    }
}
