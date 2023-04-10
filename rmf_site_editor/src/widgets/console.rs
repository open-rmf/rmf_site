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
    Hint,
    Status,
    Warning,
    Error,
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
    pub log_history: Vec<Log>,
    pub current_displayed_log: Option<Log>,
}

impl Default for Logs {
  fn default() -> Self {
      Self {
          log_history: Vec::new(),
          current_displayed_log: None,
      }
  }
}

impl Logs {
    pub fn status(&mut self, msg: &str) {
        // self.current_displayed_log = msg;
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

    pub fn get_current_status(&mut self) -> &Option<Log> {
        &self.current_displayed_log
    }

    pub fn get_log_history(&mut self) -> &Vec<Log> {
        &self.log_history
    }

    fn append_log(&mut self, log_category: LogCategory, msg: &str) {
        println!("{}", msg);
        let new_log = Log {
            category: log_category,
            message: String::from(msg),
        };
        self.current_displayed_log = Some(new_log.clone());
        if new_log.category != LogCategory::Hint {
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
        ui.label(log.message.to_string());
    });
}

pub struct ViewConsole<'a, 'w2, 's2> {
    events: &'a mut AppEvents<'w2, 's2>,
    }

    impl<'a, 'w2, 's2> ViewConsole<'a, 'w2, 's2> {
    pub fn new(events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { events }
    }

    pub fn show(mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.5;
            let status = self.events.display.logs.get_current_status();
            match status {
                Some(log) => print_log(ui, log),
                None => (),
            }
        });
        ui.add_space(5.0);
        CollapsingHeader::new("RMF Site Editor Console")
            .default_open(false)
            .show(ui, |ui| {
                //
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    if ui.button("Log History").clicked() {
                        // do something here
                    }
                });
                ui.add_space(10.);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in self.events.display.logs.get_log_history() {
                            print_log(ui, log);
                        }
                    });

                ui.add_space(10.);
            });
    }
}
