/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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
    site::*,
    widgets::AppEvents,
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, CollapsingHeader, RichText, FontId, Color32, Ui},
    EguiContext
};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogCategory {
    Status = 0,
    Warning = 1,
    Error = 2,
    Hint = 3,
    All = 4,
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogCategory::Hint => write!(f, "[HINT] "),
            LogCategory::Status => write!(f, "[STATUS] "),
            LogCategory::Warning => write!(f, "[WARNING] "),
            LogCategory::Error => write!(f, "[ERROR] "),
            _ => write!(f, "[OTHER] "),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct Log {
  pub category: LogCategory,
  pub message: String,
}

#[derive(Resource)]
pub struct Logs {
    log_history: Vec<Log>,
    current_log: Option<Log>,
    filter_category: LogCategory,
    display_limit: usize,
    show_full_history: bool,
    category_count: Vec<usize>,
}

impl Default for Logs {
  fn default() -> Self {
      Self {
          log_history: Vec::new(),
          current_log: None,
          filter_category: LogCategory::All,
          display_limit: 110,
          show_full_history: false,
          category_count: vec![0; 3], // for Status, Warning, Error
      }
  }
}

impl Logs {
    pub fn status(&mut self, msg: &str) {
        self.append_log(LogCategory::Status, msg);
    }

    pub fn hint(&mut self, msg: &str) {
        self.append_log(LogCategory::Hint, msg);
    }

    pub fn warn(&mut self, msg: &str) {
        self.append_log(LogCategory::Warning, msg);
    }

    pub fn err(&mut self, msg: &str) {
        self.append_log(LogCategory::Error, msg);
    }

    pub fn copy_log_history(&self) -> String {
        let mut output_string = String::new();

        if self.filter_category == LogCategory::All {
            for log in &self.log_history {
                output_string.push_str(&log.message);
                output_string.push_str("\n");
            }
        }
        else {
            for log in &self.log_history {
                if self.filter_category == log.category {
                    output_string.push_str(&log.message);
                    output_string.push_str("\n");
                }
            }
        }
        output_string
    }

    pub fn get_current_log(&self) -> &Option<Log> {
        &self.current_log
    }

    pub fn get_log_history(&self) -> &Vec<Log> {
        &self.log_history
    }

    pub fn get_display_limit(&self) -> &usize {
        &self.display_limit
    }

    pub fn set_display_limit(&mut self) -> &mut usize {
        &mut self.display_limit
    }

    pub fn get_filter(&self) -> &LogCategory {
        &self.filter_category
    }

    pub fn set_filter(&mut self) -> &mut LogCategory {
        &mut self.filter_category
    }

    pub fn get_show_all(&mut self) -> &bool {
        &self.show_full_history
    }

    pub fn set_show_all(&mut self, see_more: bool) {
        self.show_full_history = see_more;
    }

    pub fn get_category_count(&self) -> &Vec<usize> {
        &self.category_count
    }

    fn append_log(&mut self, log_category: LogCategory, msg: &str) {
        println!("{}", msg);
        let new_log = Log {
            category: log_category,
            message: String::from(msg),
        };
        self.current_log = Some(new_log.clone());
        if new_log.category != LogCategory::Hint {
            self.category_count[new_log.category as usize] += 1;
            self.log_history.push(new_log);
        }
    }
}

fn print_log(ui: &mut egui::Ui, log: &Log) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.5;

        // Match LogCategory to color
        let category_text_color = match log.category {
            LogCategory::Hint => Color32::LIGHT_GREEN,
            LogCategory::Status => Color32::WHITE,
            LogCategory::Warning => Color32::YELLOW,
            _ => Color32::RED,
        };
        ui.label(RichText::new(log.category.to_string()).color(category_text_color));
        // Selecting the label allows users to copy log entry to clipboard
        if ui.selectable_label(false, log.message.to_string()).clicked() {
            ui.output().copied_text = log.category.to_string() + &log.message;
        }
    });
}

pub struct ConsoleWidget<'a, 'w2, 's2> {
    events: &'a mut AppEvents<'w2, 's2>,
    }

    impl<'a, 'w2, 's2> ConsoleWidget<'a, 'w2, 's2> {
    pub fn new(events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { events }
    }

    pub fn show(mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.5;
            let status = self.events.display.logs.get_current_log();
            match status {
                Some(log) => print_log(ui, log),
                None => (),
            }
        });
        ui.add_space(5.0);
        CollapsingHeader::new("RMF Site Editor Console")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    // Filter logs by category
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.events.display.logs.set_filter()))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(40.0);
                            // Intentionally left out Hint logs as they shouldn't be saved to the log history
                            ui.selectable_value(self.events.display.logs.set_filter(), LogCategory::All, "All");
                            ui.selectable_value(self.events.display.logs.set_filter(), LogCategory::Status, "Status");
                            ui.selectable_value(self.events.display.logs.set_filter(), LogCategory::Warning, "Warning");
                            ui.selectable_value(self.events.display.logs.set_filter(), LogCategory::Error, "Error");
                        });
                    // Copy full log history to clipboard
                    if ui.button("Copy Log History").clicked() {
                        ui.output().copied_text = self.events.display.logs.copy_log_history();
                    };
                    // Slider to adjust display limit
                    ui.add(egui::Slider::new(self.events.display.logs.set_display_limit(), 10..=1000));
                });
                ui.add_space(10.);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {

                        // Show all entries
                        if *self.events.display.logs.get_show_all() {
                            // Display entries
                            for log in self.events.display.logs.get_log_history() {
                                // View all categories
                                if self.events.display.logs.get_filter() == &LogCategory::All {
                                    print_log(ui, log);
                                }
                                // View selected categories
                                else {
                                    if self.events.display.logs.get_filter() == &log.category {
                                        print_log(ui, log);
                                    }
                                }
                            }
                            // See Less button if there are too many entries
                            if self.events.display.logs.get_log_history().len() > *self.events.display.logs.get_display_limit() {
                                ui.add_space(5.0);
                                if ui.button("See Less").clicked() {
                                    // toggle to show less
                                    self.events.display.logs.set_show_all(false);
                                }
                            }
                        }

                        // Show only limited entries
                        else {
                            // Display x entries from all categories
                            if self.events.display.logs.get_filter() == &LogCategory::All {
                                // Full log history within limit, display full log history
                                if self.events.display.logs.get_log_history().len() < *self.events.display.logs.get_display_limit() {
                                    for log in self.events.display.logs.get_log_history() {
                                        print_log(ui, log);
                                    }
                                }
                                // Full log history exceeds the limit, display last xx entries
                                else {
                                    let start_idx = self.events.display.logs.get_log_history().len() - self.events.display.logs.get_display_limit();
                                    let logs_slice = &self.events.display.logs.get_log_history()[start_idx..];
                                    for log in logs_slice {
                                        print_log(ui, log);
                                    }
                                    // See more button to view full logs
                                    ui.add_space(5.0);
                                    if ui.button("See more").clicked() {
                                        // toggle to show all
                                        self.events.display.logs.set_show_all(true);
                                    }
                                }
                            }
                            // Display x entries from selected category
                            else {
                                let count = self.events.display.logs.get_category_count()[*self.events.display.logs.get_filter() as usize];
                                // Total entries from selected category doesn't exceed limit, display all entries
                                if count < *self.events.display.logs.get_display_limit() {
                                    for log in self.events.display.logs.get_log_history() {
                                        if self.events.display.logs.get_filter() == &log.category {
                                            print_log(ui, log);
                                        }
                                    }
                                }
                                // Total entries from selected category exceeds limit, display last x entries
                                else {
                                    let mut n: usize = 0;
                                    let start_idx = count - self.events.display.logs.get_display_limit();
                                    for log in self.events.display.logs.get_log_history() {
                                        // Only display logs from start index onwards
                                        if (self.events.display.logs.get_filter() == &log.category) && n >= start_idx {
                                            print_log(ui, log);
                                        }
                                        n += 1;
                                    }
                                    // See more button to view full logs
                                    ui.add_space(5.0);
                                    if ui.button("See more").clicked() {
                                        // toggle to show all
                                        self.events.display.logs.set_show_all(true);
                                    }
                                }
                            }
                        }
                    });
                ui.add_space(10.);
            });
    }
}
