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

use bevy::prelude::*;
use bevy::utils::tracing::{field::Field, Level};
use crossbeam_channel::{unbounded, Receiver, SendError, Sender};
use std::collections::HashMap;
use std::fmt::{self, Debug, Write};
use tracing_subscriber::{field::Visit, prelude::*, EnvFilter, Layer};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum LogCategory {
    Status,
    Warning,
    Error,
    Bevy,
    Hint,
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

#[derive(Debug, Clone, PartialEq, Eq, Event)]
pub struct Log {
    pub category: LogCategory,
    pub message: String,
}

impl Log {
    pub fn hint(message: String) -> Log {
        Log {
            category: LogCategory::Hint,
            message,
        }
    }

    pub fn status(message: String) -> Log {
        Log {
            category: LogCategory::Status,
            message,
        }
    }

    pub fn warn(message: String) -> Log {
        Log {
            category: LogCategory::Warning,
            message,
        }
    }

    pub fn error(message: String) -> Log {
        Log {
            category: LogCategory::Error,
            message,
        }
    }
}

pub struct LogHistoryElement {
    pub log: Log,
    pub copies: usize,
}

impl From<Log> for LogHistoryElement {
    fn from(value: Log) -> Self {
        LogHistoryElement {
            log: value,
            copies: 1,
        }
    }
}

impl fmt::Display for LogHistoryElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.copies > 1 {
            write!(
                f,
                "({}x) {}{}",
                self.copies, self.log.category, self.log.message
            )
        } else {
            write!(f, "{}{}", self.log.category, self.log.message)
        }
    }
}

#[derive(Resource)]
pub struct LogHistory {
    log_history: Vec<LogHistoryElement>,
    category_filter: HashMap<LogCategory, bool>,
    display_limit: usize,
    receiver: Receiver<Log>,
}

impl Default for LogHistory {
    fn default() -> Self {
        let mut filter_hashmap = HashMap::new();
        filter_hashmap.insert(LogCategory::Status, true);
        filter_hashmap.insert(LogCategory::Warning, true);
        filter_hashmap.insert(LogCategory::Error, true);
        filter_hashmap.insert(LogCategory::Bevy, false);
        filter_hashmap.insert(LogCategory::Hint, true);

        let (tx, rx) = unbounded();
        let tx_2 = tx.clone();
        let rx_2 = rx.clone();

        let level_name = Level::INFO;
        let filter_name = "bevy_asset=off,wgpu=error".to_string();
        let default_filter = { format!("{},{}", level_name, filter_name) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&default_filter))
            .unwrap();

        let subscriber = tracing_subscriber::registry().with(LogSubscriber { sender: tx_2 });
        let subscriber = subscriber.with(filter_layer);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let fmt_layer = tracing_subscriber::fmt::Layer::default();
            let subscriber = subscriber.with(fmt_layer);
            tracing::subscriber::set_global_default(subscriber).ok();
        }
        #[cfg(target_arch = "wasm32")]
        {
            tracing::subscriber::set_global_default(subscriber);
        }

        Self {
            log_history: Vec::new(),
            category_filter: filter_hashmap,
            display_limit: 100,
            receiver: rx_2,
        }
    }
}

impl LogHistory {
    pub fn copy_log_history(&self) -> String {
        let mut output_string = String::new();

        for element in &self.log_history {
            if *self.category_filter.get(&element.log.category).unwrap() {
                output_string.push_str(&element.to_string());
                output_string.push_str("\n");
            }
        }
        output_string
    }

    pub fn log_history(&self) -> &Vec<LogHistoryElement> {
        &self.log_history
    }

    pub fn iter(&self) -> impl Iterator<Item = &LogHistoryElement> {
        self.log_history
            .iter()
            .rev()
            .filter(|e| self.category_filter[&e.log.category])
            .take(self.display_limit)
    }

    pub fn category_present(&self, category: &LogCategory) -> &bool {
        self.category_filter.get(category).unwrap()
    }

    pub fn category_present_mut(&mut self, category: LogCategory) -> &mut bool {
        self.category_filter.get_mut(&category).unwrap()
    }

    pub fn display_limit(&self) -> usize {
        self.display_limit
    }

    pub fn display_limit_mut(&mut self) -> &mut usize {
        &mut self.display_limit
    }

    pub fn receive_logs(&mut self) {
        for msg in self.receiver.try_iter().collect::<Vec<_>>() {
            self.push(msg);
        }
    }

    pub fn push(&mut self, log: Log) {
        if let Some(last) = self.log_history.last_mut() {
            if last.log == log {
                last.copies += 1;
            } else {
                self.log_history.push(log.into());
            }
        } else {
            self.log_history.push(log.into());
        }
    }

    pub fn top(&self) -> Option<&LogHistoryElement> {
        for element in self.log_history.iter().rev() {
            if self.category_filter[&element.log.category] {
                return Some(element);
            }
        }

        None
    }

    pub fn all_categories_are_selected(&self) -> bool {
        for selected in self.category_filter.values() {
            if !selected {
                return false;
            }
        }

        return true;
    }

    pub fn select_all_categories(&mut self) {
        for selected in self.category_filter.values_mut() {
            *selected = true;
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

impl<S> Layer<S> for LogSubscriber
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut recorder = LogRecorder::new();
        event.record(&mut recorder);

        // Default category
        let message = format!("{}", recorder);

        // Check if this is a Bevy or RMF Site log
        let category = if event.metadata().target().contains("bevy") {
            LogCategory::Bevy
        } else {
            match *event.metadata().level() {
                Level::INFO => LogCategory::Status,
                Level::WARN => LogCategory::Warning,
                Level::ERROR => LogCategory::Error,
                _ => LogCategory::Error,
            }
        };

        let log = Log { category, message };
        let send_message = self.sender.send(log);
        match send_message {
            Ok(()) => send_message.unwrap(),
            Err(SendError(e)) => println!("Unable to send log: {:?}", e),
        }
    }
}

fn receive_logs(mut log_history: ResMut<LogHistory>, mut log_events: EventReader<Log>) {
    log_history.receive_logs();
    for log in log_events.read() {
        log_history.push(log.clone());
    }
}

pub struct LogHistoryPlugin;

impl Plugin for LogHistoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Log>()
            .init_resource::<LogHistory>()
            .add_systems(Update, receive_logs);
    }
}
