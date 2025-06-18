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
use bevy_egui::egui::{self, CollapsingHeader, Color32, RichText};

/// This widget provides a console that displays information, warning, and error
/// messages.
#[derive(Default)]
pub struct ConsoleWidgetPlugin {}

impl Plugin for ConsoleWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogHistory>();
        let widget = PanelWidget::new(console_widget, app.world_mut());
        app.world_mut().spawn(widget);
    }
}

fn console_widget(In(input): In<PanelWidgetInput>, mut log_history: ResMut<LogHistory>) {
    egui::TopBottomPanel::bottom("log_consolse")
        .resizable(true)
        .min_height(30.0)
        .max_height(300.0)
        .show(&input.context, |ui| {
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
                        let mut all_are_checked = log_history.all_categories_are_selected();
                        let all_were_checked = all_are_checked;
                        ui.checkbox(&mut all_are_checked, "All");
                        // Use strum crate for iterating through enum and converting to string?
                        if let Some(checked_warning) =
                            log_history.category_present_mut(LogCategory::Warning)
                        {
                            ui.checkbox(checked_warning, "Warning");
                        };
                        if let Some(checked_error) =
                            log_history.category_present_mut(LogCategory::Error)
                        {
                            ui.checkbox(checked_error, "Error");
                        };
                        if let Some(checked_bevy) =
                            log_history.category_present_mut(LogCategory::Bevy)
                        {
                            ui.checkbox(checked_bevy, "Bevy");
                        };
                        // Copy full log history to clipboard
                        if ui.button("Copy Log History").clicked() {
                            ui.ctx().copy_text(log_history.copy_log_history());
                        }
                        // Slider to adjust display limit
                        let history_size = log_history.log_history().len() as f64;
                        let nearest_hundred: usize = 100 * (history_size / 100.0).ceil() as usize;
                        ui.add(egui::Slider::new(
                            log_history.display_limit_mut(),
                            10..=nearest_hundred,
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
            ui.ctx()
                .copy_text(element.log.category.to_string() + &element.log.message);
        }

        if truncated {
            ui.label(" [...]").on_hover_text(
                "Some of the message is hidden. Click on it to copy the \
                    full text to your clipboard.",
            );
        }
    });
}
