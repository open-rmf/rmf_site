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

use crate::{log::*, widgets::prelude::*};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, CollapsingHeader, Color32, RichText},
    EguiContexts,
};

#[derive(Default)]
pub struct ConsoleWidgetPlugin {

}

impl Plugin for ConsoleWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogHistory>();
        let widget = PanelWidget::new(console_widget, &mut app.world);
        app.world.spawn(widget);
    }
}

fn console_widget(
    In(_): In<Entity>,
    mut log_history: ResMut<LogHistory>,
    mut egui_context: EguiContexts,
) {
    egui::TopBottomPanel::bottom("log_consolse")
        .resizable(true)
        .min_height(30.0)
        .max_height(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.5;
                let status = log_history.top();
                if let Some(log) = status {
                    print_log(ui, log);
                }
            });
            ui.add_space(5.0);
            CollapsingHeader::new("Log Console")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        // Filter logs by category
                        let mut all_are_checked = log_history
                            .all_categories_are_selected();
                        let all_were_checked = all_are_checked;
                        ui.checkbox(&mut all_are_checked, "All");
                        ui.checkbox(
                            log_history.category_present_mut(LogCategory::Status),
                            "Status",
                        );
                        ui.checkbox(
                            log_history.category_present_mut(LogCategory::Warning),
                            "Warning",
                        );
                        ui.checkbox(
                            log_history.category_present_mut(LogCategory::Error),
                            "Error",
                        );
                        ui.checkbox(
                            log_history.category_present_mut(LogCategory::Bevy),
                            "Bevy",
                        );
                        // Copy full log history to clipboard
                        if ui.button("Copy Log History").clicked() {
                            ui.output_mut(|o| {
                                o.copied_text = log_history.copy_log_history();
                            });
                        }
                        // Slider to adjust display limit
                        // TODO(@mxgrey): Consider allowing this range to
                        // automatically grow/shrink when the selected value
                        // approaches or leaves the upper limit.
                        ui.add(egui::Slider::new(
                            log_history.display_limit_mut(),
                            10..=1000,
                        ));

                        if !all_were_checked && all_are_checked {
                            log_history.select_all_categories();
                        }
                    });
                    ui.add_space(10.0);

                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            let mut count = 0;
                            for element in log_history.iter() {
                                print_log(ui, element);
                                count += 1;
                            }
                            if count >= log_history.display_limit() {
                                ui.add_space(5.0);
                                if ui.button("See more").clicked() {
                                    *log_history.display_limit_mut() += 100;
                                }
                            }
                        });
                    ui.add_space(10.0);
                });
        });
}

fn print_log(ui: &mut egui::Ui, element: &LogHistoryElement) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.5;

        if element.copies > 1 {
            ui.label(RichText::new(format!("({}x) ", element.copies)).color(Color32::GOLD));
        }
        // Match LogCategory to color
        let category_text_color = match element.log.category {
            LogCategory::Hint => Color32::LIGHT_GREEN,
            LogCategory::Status => Color32::WHITE,
            LogCategory::Warning => Color32::YELLOW,
            LogCategory::Error => Color32::RED,
            LogCategory::Bevy => Color32::LIGHT_BLUE,
        };

        let mut truncated = false;
        let msg = if element.log.message.len() > 80 {
            truncated = true;
            &element.log.message[..80]
        } else {
            &element.log.message
        };

        let msg = if let Some(nl) = msg.find("\n") {
            truncated = true;
            &msg[..nl]
        } else {
            msg
        };

        ui.label(RichText::new(element.log.category.to_string()).color(category_text_color));
        // Selecting the label allows users to copy log entry to clipboard
        if ui.selectable_label(false, msg).clicked() {
            ui.output_mut(|o| {
                o.copied_text = element.log.category.to_string() + &element.log.message
            });
        }

        if truncated {
            ui.label(" [...]").on_hover_text(
                "Some of the message is hidden. Click on it to copy the \
                    full text to your clipboard.",
            );
        }
    });
}
