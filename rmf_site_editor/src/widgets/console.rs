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

use crate::{site::*, widgets::AppEvents};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, CollapsingHeader, Color32, FontId, RichText, Ui},
    EguiContext,
};
use std::fmt::{self, Write};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum LogCategory {
    Status = 0,
    Warning = 1,
    Error = 2,
    Hint = 3,
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogCategory::Hint => write!(f, "[HINT] "),
            LogCategory::Status => write!(f, "[STATUS] "),
            LogCategory::Warning => write!(f, "[WARNING] "),
            LogCategory::Error => write!(f, "[ERROR] "),
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
    filter_map: HashMap::<LogCategory, bool>,
    checked_all: bool, // True if "All" box is checked
    stored_checked_all: bool, // Stored state of "All" checkbox
    display_limit: usize,
    show_full_history: bool,
    category_count: Vec<usize>,
}

impl Default for Logs {
    fn default() -> Self {
        let mut filter_hashmap = HashMap::new();
        filter_hashmap.insert(LogCategory::Status, true);
        filter_hashmap.insert(LogCategory::Warning, true);
        filter_hashmap.insert(LogCategory::Error, true);

        Self {
            log_history: Vec::new(),
            current_log: None,
            filter_map: filter_hashmap,
            checked_all: true,
            stored_checked_all: true,
            display_limit: 100,
            show_full_history: false,
            category_count: vec![0; 3], // for Status, Warning, Error
        }
    }
}

pub trait FormatInput {
    fn format_to_string(&self) -> String;
}

impl FormatInput for &str {
    fn format_to_string(&self) -> String {
        self.to_string()
    }
}

impl FormatInput for fmt::Arguments<'_> {
    fn format_to_string(&self) -> String {
        let mut s = String::new();
        let ag = &self;
        write!(&mut s, "{ag}");
        s
    }
}

impl Logs {
    pub fn status<T: FormatInput>(&mut self, args: T) {
        let msg = args.format_to_string();
        self.append_log(LogCategory::Status, msg);
    }

    pub fn hint<T: FormatInput>(&mut self, args: T) {
        let msg = args.format_to_string();
        self.append_log(LogCategory::Hint, msg);
    }

    pub fn warn<T: FormatInput>(&mut self, args: T) {
        let msg = args.format_to_string();
        self.append_log(LogCategory::Warning, msg);
    }

    pub fn err<T: FormatInput>(&mut self, args: T) {
        let msg = args.format_to_string();
        self.append_log(LogCategory::Error, msg);
    }

    pub fn copy_log_history(&self) -> String {
        let mut output_string = String::new();

        for log in &self.log_history {
            if *self.filter_map.get(&log.category).unwrap() {
                output_string.push_str(&log.category.to_string());
                output_string.push_str(&log.message);
                output_string.push_str("\n");
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

    pub fn checked_all_mut(&mut self) -> &mut bool {
        // If "All" checkbox is freshly clicked
        if self.stored_checked_all != self.checked_all {
            self.stored_checked_all = self.checked_all;
            if self.checked_all {
                for (cat, present) in &mut self.filter_map {
                    *present = true;
                }
            }
        }
        &mut self.checked_all
    }

    pub fn category_present(&self, category: &LogCategory) -> &bool {
        self.filter_map.get(category).unwrap()
    }

    pub fn category_present_mut(&mut self, category: LogCategory) -> &mut bool {
        self.filter_map.get_mut(&category).unwrap()
    }

    pub fn displayed_category_count(&mut self) -> usize {
        let mut total_count: usize = 0;
        for (category, present) in &self.filter_map {
            if *present {
                total_count += &self.category_count[*category as usize];
            }
        }
        total_count
    }

    pub fn get_display_limit(&self) -> &usize {
        &self.display_limit
    }

    pub fn set_display_limit(&mut self) -> &mut usize {
        &mut self.display_limit
    }

    pub fn get_show_all(&mut self) -> &bool {
        &self.show_full_history
    }

    pub fn set_show_all(&mut self, see_more: bool) {
        self.show_full_history = see_more;
    }

    pub fn update_filter(&mut self) {
        let mut all_categories_present = true;
        for (cat, present) in &mut self.filter_map {
            if !*present && self.stored_checked_all {
                self.checked_all = false;
                self.stored_checked_all = false;
                return;
            } else if !*present {
                all_categories_present = false;
            }
        }
        if all_categories_present && !self.stored_checked_all {
            self.checked_all = true;
            self.stored_checked_all = true;
        }
    }

    fn append_log(&mut self, log_category: LogCategory, msg: String) {
        println!("{}", msg);
        let new_log = Log {
            category: log_category,
            message: msg,
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
            LogCategory::Error => Color32::RED,
        };
        ui.label(RichText::new(log.category.to_string()).color(category_text_color));
        // Selecting the label allows users to copy log entry to clipboard
        if ui
            .selectable_label(false, log.message.to_string())
            .clicked()
        {
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
                    ui.checkbox(self.events.display.logs.checked_all_mut(), "All");
                    ui.checkbox(self.events.display.logs.category_present_mut(LogCategory::Status), "Status");
                    ui.checkbox(self.events.display.logs.category_present_mut(LogCategory::Warning), "Warning");
                    ui.checkbox(self.events.display.logs.category_present_mut(LogCategory::Error), "Error");
                    // Copy full log history to clipboard
                    if ui.button("Copy Log History").clicked() {
                        ui.output().copied_text = self.events.display.logs.copy_log_history();
                    };
                    // Slider to adjust display limit
                    ui.add(egui::Slider::new(
                        self.events.display.logs.set_display_limit(),
                        10..=1000,
                    ));
                    self.events.display.logs.update_filter();
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
                                if *self.events.display.logs.category_present(&log.category) {
                                    print_log(ui, log);
                                }
                            }
                            // See Less button if there are too many entries
                            if self.events.display.logs.get_log_history().len()
                                > *self.events.display.logs.get_display_limit()
                            {
                                ui.add_space(5.0);
                                if ui.button("See Less").clicked() {
                                    // toggle to show less
                                    self.events.display.logs.set_show_all(false);
                                }
                            }
                        }
                        // Show only limited entries
                        else {
                            let count = self.events.display.logs.displayed_category_count();
                            // Total entries don't exceed limit, display all entries
                            if count < *self.events.display.logs.get_display_limit() {
                                for log in self.events.display.logs.get_log_history() {
                                    if *self.events.display.logs.category_present(&log.category) {
                                        print_log(ui, log);
                                    }
                                }
                            }
                            // Total entries exceed limit, display last x entries
                            else {
                                //
                                let mut n: usize = 0;
                                let start_idx = count - self.events.display.logs.get_display_limit();
                                for log in self.events.display.logs.get_log_history() {
                                    // Only display logs from start index onwards
                                    if *self.events.display.logs.category_present(&log.category) && n >= start_idx {
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
                    });
                ui.add_space(10.);
            });
    }
}
