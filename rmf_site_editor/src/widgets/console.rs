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
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    egui::{self, CollapsingHeader, Color32, FontId, RichText, Ui},
    EguiContext,
};
use std::collections::HashMap;
use std::fmt::{self, Debug, Write};
use bevy_utils::tracing::{
    field::Field,
    span::Record,
    Event, Id, Level, Subscriber,
};
use crossbeam_channel::{unbounded, Sender, Receiver, SendError, TryRecvError};
use tracing_subscriber::{field::Visit, layer::Context, prelude::*, EnvFilter, Layer};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum LogCategory {
    Status = 0,
    Warning = 1,
    Error = 2,
    Bevy = 3,
    Hint = 4,
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogCategory::Hint => write!(f, "[HINT] "),
            LogCategory::Status => write!(f, "[STATUS] "),
            LogCategory::Warning => write!(f, "[WARNING] "),
            LogCategory::Error => write!(f, "[ERROR] "),
            LogCategory::Bevy => write!(f, "[BEVY] "),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct Log {
    pub category: LogCategory,
    pub message: String,
}

#[derive(Resource)]
pub struct LogHistory {
    log_history: Vec<Log>,
    current_log: Option<Log>,
    category_filter: HashMap<LogCategory, bool>,
    checked_all: bool,        // True if "All" box is checked
    stored_checked_all: bool, // Stored state of "All" checkbox
    display_limit: usize,
    show_full_history: bool,
    category_count: Vec<usize>,
    receiver: Receiver<Log>,
}

impl Default for LogHistory {
    fn default() -> Self {
        let mut filter_hashmap = HashMap::new();
        filter_hashmap.insert(LogCategory::Status, true);
        filter_hashmap.insert(LogCategory::Warning, true);
        filter_hashmap.insert(LogCategory::Error, true);
        filter_hashmap.insert(LogCategory::Bevy, false);

        let (tx, rx) = unbounded();
        let tx_2 = tx.clone();
        let rx_2 = rx.clone();

        let level_name = Level::INFO;
        let filter_name = "wgpu=error".to_string();
        let default_filter = { format!("{},{}", level_name, filter_name) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&default_filter))
            .unwrap();

        let subscriber = tracing_subscriber::registry().with(LogSubscriber { sender: tx_2 });
        let subscriber = subscriber.with(filter_layer);
        tracing::subscriber::set_global_default(subscriber);

        Self {
            log_history: Vec::new(),
            current_log: None,
            category_filter: filter_hashmap,
            checked_all: true,
            stored_checked_all: true,
            display_limit: 100,
            show_full_history: false,
            category_count: vec![0; 4], // for Status, Warning, Error, Bevy
            receiver: rx_2,
        }
    }
}

impl LogHistory {
    pub fn copy_log_history(&self) -> String {
        let mut output_string = String::new();

        for log in &self.log_history {
            if *self.category_filter.get(&log.category).unwrap() {
                output_string.push_str(&log.category.to_string());
                output_string.push_str(&log.message);
                output_string.push_str("\n");
            }
        }
        output_string
    }

    pub fn current_log(&self) -> &Option<Log> {
        &self.current_log
    }

    pub fn log_history(&self) -> &Vec<Log> {
        &self.log_history
    }

    pub fn checked_all_mut(&mut self) -> &mut bool {
        // If "All" checkbox is freshly clicked
        if self.stored_checked_all != self.checked_all {
            self.stored_checked_all = self.checked_all;
            if self.checked_all {
                for (cat, present) in &mut self.category_filter {
                    *present = true;
                }
            }
        }
        &mut self.checked_all
    }

    pub fn category_present(&self, category: &LogCategory) -> &bool {
        self.category_filter.get(category).unwrap()
    }

    pub fn category_present_mut(&mut self, category: LogCategory) -> &mut bool {
        self.category_filter.get_mut(&category).unwrap()
    }

    pub fn displayed_category_count(&mut self) -> usize {
        let mut total_count: usize = 0;
        for (category, present) in &self.category_filter {
            if *present {
                total_count += &self.category_count[*category as usize];
            }
        }
        total_count
    }

    pub fn display_limit(&self) -> &usize {
        &self.display_limit
    }

    pub fn display_limit_mut(&mut self) -> &mut usize {
        &mut self.display_limit
    }

    pub fn receive_log(&mut self) {
        match self.receiver.try_recv() {
            Ok(msg) => self.append_log(msg),
            Err(TryRecvError::Disconnected) => println!("Unable to receive log: Disconnected"),
            Err(TryRecvError::Empty) => (),
        }
    }

    pub fn show_all(&self) -> bool {
        self.show_full_history
    }

    pub fn show_all_mut(&mut self, see_more: bool) {
        self.show_full_history = see_more;
    }

    pub fn update_filter(&mut self) {
        let mut all_categories_present = true;
        for (cat, present) in &mut self.category_filter {
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

    fn append_log(&mut self, log: Log) {
        self.current_log = Some(log.clone());
        if log.category != LogCategory::Hint {
            self.category_count[log.category as usize] += 1;
            self.log_history.push(log);
        }
    }
}

struct LogRecorder(String, bool);
impl LogRecorder {
    fn new() -> Self {
        LogRecorder(String::new(), false)
    }
}

impl Visit for LogRecorder {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            if !self.0.is_empty() {
                self.0 = format!("{:?}\n{}", value, self.0)
            } else {
                self.0 = format!("{:?}", value)
            }
        } else {
            if self.1 {
                // following args
                write!(self.0, " ").unwrap();
            } else {
                // first arg
                self.1 = true;
            }
            write!(self.0, "{} = {:?};", field.name(), value).unwrap();
        }
    }
}

impl fmt::Display for LogRecorder {
    fn fmt(&self, mut f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.0.is_empty() {
            write!(&mut f, " {}", self.0)
        } else {
            Ok(())
        }
    }
}

pub struct LogSubscriber {
    sender: Sender<Log>,
}


impl<S> Layer<S> for LogSubscriber where S: tracing::Subscriber {
    //
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut recorder = LogRecorder::new();
        event.record(&mut recorder);

        // Default category
        let mut category = LogCategory::Status;
        let message = format!("{}\0", recorder);

        // Check if this is a Bevy or RMF Site log
        if event.metadata().target().contains("bevy") {
            category = LogCategory::Bevy;
        } else {
            category = match *event.metadata().level() {
                Level::INFO => LogCategory::Status,
                Level::WARN => LogCategory::Warning,
                Level::ERROR => LogCategory::Error,
                _ => LogCategory::Hint,
            };
        }

        let log = Log {
            category: category,
            message: message,
        };

        let send_message = self.sender.send(log);
        match send_message {
            Ok(()) => send_message.unwrap(),
            Err(SendError(e)) => println!("Unable to send log: {:?}", e),
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
            LogCategory::Bevy => Color32::LIGHT_BLUE,
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

    pub fn show(self, ui: &mut Ui) {
        // Check updated logs
        self.events.display.log_history.receive_log();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.5;
            let status = self.events.display.log_history.current_log();
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
                    ui.checkbox(self.events.display.log_history.checked_all_mut(), "All");
                    ui.checkbox(
                        self.events
                            .display
                            .log_history
                            .category_present_mut(LogCategory::Status),
                        "Status",
                    );
                    ui.checkbox(
                        self.events
                            .display
                            .log_history
                            .category_present_mut(LogCategory::Warning),
                        "Warning",
                    );
                    ui.checkbox(
                        self.events
                            .display
                            .log_history
                            .category_present_mut(LogCategory::Error),
                        "Error",
                    );
                    ui.checkbox(
                        self.events
                            .display
                            .log_history
                            .category_present_mut(LogCategory::Bevy),
                        "Bevy",
                    );
                    // Copy full log history to clipboard
                    if ui.button("Copy Log History").clicked() {
                        ui.output().copied_text =
                            self.events.display.log_history.copy_log_history();
                    };
                    // Slider to adjust display limit
                    ui.add(egui::Slider::new(
                        self.events.display.log_history.display_limit_mut(),
                        10..=1000,
                    ));
                    self.events.display.log_history.update_filter();
                });
                ui.add_space(10.);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        // Show all entries
                        if self.events.display.log_history.show_all() {
                            // Display entries
                            for log in self.events.display.log_history.log_history() {
                                if *self
                                    .events
                                    .display
                                    .log_history
                                    .category_present(&log.category)
                                {
                                    print_log(ui, log);
                                }
                            }
                            // See Less button if there are too many entries
                            if self.events.display.log_history.log_history().len()
                                > *self.events.display.log_history.display_limit()
                            {
                                ui.add_space(5.0);
                                if ui.button("See Less").clicked() {
                                    // toggle to show less
                                    self.events.display.log_history.show_all_mut(false);
                                }
                            }
                        }
                        // Show only limited entries
                        else {
                            let count = self.events.display.log_history.displayed_category_count();
                            // Total entries don't exceed limit, display all entries
                            if count < *self.events.display.log_history.display_limit() {
                                for log in self.events.display.log_history.log_history() {
                                    if *self
                                        .events
                                        .display
                                        .log_history
                                        .category_present(&log.category)
                                    {
                                        print_log(ui, log);
                                    }
                                }
                            }
                            // Total entries exceed limit, display last x entries
                            else {
                                //
                                let mut n: usize = 0;
                                let start_idx =
                                    count - self.events.display.log_history.display_limit();
                                for log in self.events.display.log_history.log_history() {
                                    // Only display logs from start index onwards
                                    if *self
                                        .events
                                        .display
                                        .log_history
                                        .category_present(&log.category)
                                        && n >= start_idx
                                    {
                                        print_log(ui, log);
                                    }
                                    n += 1;
                                }
                                // See more button to view full logs
                                ui.add_space(5.0);
                                if ui.button("See more").clicked() {
                                    // toggle to show all
                                    self.events.display.log_history.show_all_mut(true);
                                }
                            }
                        }
                    });
                ui.add_space(10.);
            });
    }
}
