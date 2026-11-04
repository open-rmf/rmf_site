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

use crate::{
    site::{
        Change, ConnectSelectedCells, Delete, ExpandNavGrid, GridExpansionDirection,
        MergeSelectedCells, SnapAndStitchCells,
    },
    widgets::{inspector::InspectAngle, prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, ComboBox, DragValue, RichText, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{
    Angle, Edge, GridCellCoord, NavCellEdgeMarker, NavCellMarker, NavGuidePreference, Path,
    PreferredDirection, RecallPreferredDirection,
};
use rmf_site_picking::Select;

#[derive(Default)]
pub struct InspectNavCellPlugin {}

impl Plugin for InspectNavCellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavCellToolsSettings>().add_plugins((
            InspectionPlugin::<InspectNavCell>::new(),
            InspectionPlugin::<InspectNavCellEdge>::new(),
        ));
    }
}

#[derive(Resource, Debug, Clone)]
pub struct NavCellToolsSettings {
    pub snap_distance: f32,
}

impl Default for NavCellToolsSettings {
    fn default() -> Self {
        Self {
            snap_distance: 0.45,
        }
    }
}

#[derive(SystemParam)]
pub struct InspectNavCell<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub cells: Query<
        'w,
        's,
        (
            &'static Path<Entity>,
            &'static PreferredDirection,
            Option<&'static RecallPreferredDirection>,
            &'static GridCellCoord<Entity>,
        ),
        With<NavCellMarker>,
    >,
    pub all_grid_cells:
        Query<'w, 's, (Entity, &'static GridCellCoord<Entity>), With<NavCellMarker>>,
    pub expand_grid: EventWriter<'w, ExpandNavGrid>,
    pub snap_stitch: EventWriter<'w, SnapAndStitchCells>,
    pub merge_cells: EventWriter<'w, MergeSelectedCells>,
    pub connect_cells: EventWriter<'w, ConnectSelectedCells>,
    pub delete: EventWriter<'w, Delete>,
    pub select: EventWriter<'w, Select>,
    pub settings: ResMut<'w, NavCellToolsSettings>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectNavCell<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_single(selection, ui);
    }
}

impl<'w, 's> InspectNavCell<'w, 's> {
    pub fn show_single(&mut self, selection: Entity, ui: &mut Ui) {
        let Ok((path, pref_dir, recall, grid_coord)) = self.cells.get(selection) else {
            return;
        };
        let path_len = path.len();
        let current_dir = *pref_dir;
        let recalled_angle = recall.and_then(|r| r.angle).unwrap_or(Angle::Deg(0.0));
        let grid_idx_opt = grid_coord.0;

        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("Polygon Vertices: {}", path_len))
                .color(egui::Color32::from_gray(190)),
        );
        if let Some(idx) = grid_idx_opt {
            ui.horizontal(|ui| {
                ui.label(format!("Grid Cell: (row = {}, col = {})", idx.row, idx.col));
                if ui
                    .small_button("Select Entire Grid")
                    .on_hover_text("Highlight all cells belonging to this NavGrid")
                    .clicked()
                {
                    let grid_cells: Vec<Entity> = self
                        .all_grid_cells
                        .iter()
                        .filter_map(|(e, gc)| gc.0.filter(|g| g.grid == idx.grid).map(|_| e))
                        .collect();
                    for (i, cell_e) in grid_cells.into_iter().enumerate() {
                        self.select
                            .write(Select::new(Some(cell_e)).multi_select(i > 0));
                    }
                }
            });
        }

        self.show_batch(&[selection], Some((current_dir, recalled_angle)), ui);
    }

    /// Shared UI for both single-selected and multi-selected `NavCell`s.
    pub fn show_batch(
        &mut self,
        selected_cells: &[Entity],
        single_dir_info: Option<(PreferredDirection, Angle)>,
        ui: &mut Ui,
    ) {
        if selected_cells.is_empty() {
            return;
        }

        // Determine current direction across selection
        let (common_dir, mut edit_angle) = if let Some((dir, recalled)) = single_dir_info {
            let angle = dir.angle().unwrap_or(recalled);
            (Some(dir), angle)
        } else {
            let mut first_dir: Option<PreferredDirection> = None;
            let mut all_same = true;
            for &cell_e in selected_cells {
                if let Ok((_, dir, _, _)) = self.cells.get(cell_e) {
                    if let Some(f) = first_dir {
                        if f != *dir {
                            all_same = false;
                        }
                    } else {
                        first_dir = Some(*dir);
                    }
                }
            }
            let dir = if all_same { first_dir } else { None };
            let angle = first_dir.and_then(|d| d.angle()).unwrap_or(Angle::Deg(0.0));
            (dir, angle)
        };

        ui.separator();
        ui.label(RichText::new("Preferred Direction").strong());
        ui.horizontal(|ui| {
            let mut new_dir: Option<PreferredDirection> = None;
            if ui
                .selectable_label(common_dir == Some(PreferredDirection::None), "None")
                .on_hover_text("Omnidirectional cell (no directional bias)")
                .clicked()
            {
                new_dir = Some(PreferredDirection::None);
            }
            if ui
                .selectable_label(
                    common_dir == Some(PreferredDirection::north()),
                    "↑ North (90°)",
                )
                .clicked()
            {
                new_dir = Some(PreferredDirection::north());
            }
            if ui
                .selectable_label(
                    common_dir == Some(PreferredDirection::south()),
                    "↓ South (-90°)",
                )
                .clicked()
            {
                new_dir = Some(PreferredDirection::south());
            }
            if ui
                .selectable_label(
                    common_dir == Some(PreferredDirection::east()),
                    "→ East (0°)",
                )
                .clicked()
            {
                new_dir = Some(PreferredDirection::east());
            }
            if ui
                .selectable_label(
                    common_dir == Some(PreferredDirection::west()),
                    "← West (180°)",
                )
                .clicked()
            {
                new_dir = Some(PreferredDirection::west());
            }

            if let Some(target_dir) = new_dir {
                for &cell_e in selected_cells {
                    self.commands.trigger(Change::new(target_dir, cell_e));
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Custom Angle:");
            let prev_angle = edit_angle;
            InspectAngle::new(&mut edit_angle).show(ui);
            if edit_angle != prev_angle {
                let target_dir = PreferredDirection::Angle(edit_angle);
                for &cell_e in selected_cells {
                    self.commands.trigger(Change::new(target_dir, cell_e));
                }
            }
            if ui
                .button("⇄ Flip 180°")
                .on_hover_text("Reverse preferred direction of selected cells by 180°")
                .clicked()
            {
                for &cell_e in selected_cells {
                    if let Ok((_, dir, _, _)) = self.cells.get(cell_e) {
                        if let PreferredDirection::Angle(a) = dir {
                            let flipped_deg = (a.degrees() + 180.0).rem_euclid(360.0);
                            let normalized = if flipped_deg > 180.0 {
                                flipped_deg - 360.0
                            } else {
                                flipped_deg
                            };
                            self.commands.trigger(Change::new(
                                PreferredDirection::Angle(Angle::Deg(normalized)),
                                cell_e,
                            ));
                        }
                    }
                }
            }
        });

        // Grid Expansion Controls (shown if any selected cell belongs to a NavGrid)
        let has_grid = selected_cells
            .iter()
            .any(|&c| self.cells.get(c).is_ok_and(|(_, _, _, gc)| gc.0.is_some()));
        if has_grid {
            ui.separator();
            ui.label(RichText::new("Expand Grid Rows / Columns").strong());
            ui.horizontal(|ui| {
                if ui
                    .button("➕ Row North (+Y)")
                    .on_hover_text("Add a row of cells to the North boundary of the grid")
                    .clicked()
                {
                    self.expand_grid.write(ExpandNavGrid {
                        cells: selected_cells.to_vec(),
                        direction: GridExpansionDirection::North,
                    });
                }
                if ui
                    .button("➕ Row South (-Y)")
                    .on_hover_text("Add a row of cells to the South boundary of the grid")
                    .clicked()
                {
                    self.expand_grid.write(ExpandNavGrid {
                        cells: selected_cells.to_vec(),
                        direction: GridExpansionDirection::South,
                    });
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button("➕ Col East (+X)")
                    .on_hover_text("Add a column of cells to the East boundary of the grid")
                    .clicked()
                {
                    self.expand_grid.write(ExpandNavGrid {
                        cells: selected_cells.to_vec(),
                        direction: GridExpansionDirection::East,
                    });
                }
                if ui
                    .button("➕ Col West (-X)")
                    .on_hover_text("Add a column of cells to the West boundary of the grid")
                    .clicked()
                {
                    self.expand_grid.write(ExpandNavGrid {
                        cells: selected_cells.to_vec(),
                        direction: GridExpansionDirection::West,
                    });
                }
            });
        }

        // Multi-cell Joining, Snapping & Merging Operations
        if selected_cells.len() >= 2 {
            ui.separator();
            ui.label(RichText::new("Join & Merge Polygon Cells").strong());
            ui.horizontal(|ui| {
                ui.label("Snap Distance (m):");
                ui.add(
                    DragValue::new(&mut self.settings.snap_distance)
                        .speed(0.02)
                        .range(0.01..=5.0),
                );
            });

            ui.horizontal(|ui| {
                if ui
                    .button("🧲 Snap & Stitch Grids")
                    .on_hover_text(
                        "Snap nearby boundary anchors across misaligned grids and connect adjacent cells",
                    )
                    .clicked()
                {
                    self.snap_stitch.write(SnapAndStitchCells {
                        cells: selected_cells.to_vec(),
                        snap_distance: self.settings.snap_distance,
                    });
                }

                if ui
                    .button("⛙ Merge into Single Polygon")
                    .on_hover_text(
                        "Union selected cells into one larger polygon cell and rewire graph edges",
                    )
                    .clicked()
                {
                    self.merge_cells.write(MergeSelectedCells {
                        cells: selected_cells.to_vec(),
                        snap_distance: self.settings.snap_distance,
                    });
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .button("🔗 Connect Cells")
                    .on_hover_text("Create NavCellEdge connections between selected cells")
                    .clicked()
                {
                    self.connect_cells.write(ConnectSelectedCells {
                        cells: selected_cells.to_vec(),
                        connect: true,
                    });
                }
                if ui
                    .button("✂ Disconnect Cells")
                    .on_hover_text("Remove NavCellEdge connections between selected cells")
                    .clicked()
                {
                    self.connect_cells.write(ConnectSelectedCells {
                        cells: selected_cells.to_vec(),
                        connect: false,
                    });
                }
            });
        }

        ui.separator();
        if ui
            .button("🗑 Delete Selected Cell(s)")
            .on_hover_text("Delete selected NavCell(s) and their incident NavCellEdges")
            .clicked()
        {
            for &cell_e in selected_cells {
                self.delete.write(Delete::new(cell_e).and_dependents());
            }
        }
    }
}

#[derive(SystemParam)]
pub struct InspectNavCellEdge<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub edges: Query<
        'w,
        's,
        (&'static Edge<Entity>, &'static NavGuidePreference),
        With<NavCellEdgeMarker>,
    >,
    pub delete: EventWriter<'w, Delete>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectNavCellEdge<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Ok((_, guide)) = params.edges.get(selection) else {
            return;
        };
        let mut current_guide = *guide;

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Guide Preference:");
            ComboBox::from_id_salt("nav_cell_edge_guide_pref")
                .selected_text(current_guide.label())
                .show_ui(ui, |ui| {
                    for variant in [
                        NavGuidePreference::Auto,
                        NavGuidePreference::Forward,
                        NavGuidePreference::Backward,
                        NavGuidePreference::None,
                    ] {
                        ui.selectable_value(&mut current_guide, variant, variant.label());
                    }
                });
        });

        if current_guide != *guide {
            params
                .commands
                .trigger(Change::new(current_guide, selection));
        }

        ui.add_space(4.0);
        if ui.button("🗑 Delete Edge").clicked() {
            params.delete.write(Delete::new(selection));
        }
    }
}
