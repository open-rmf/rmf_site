/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Component, Reflect};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct TaskParams {
    #[serde(default, skip_serializing_if = "is_default")]
    pub unix_millis_earliest_start_time: Option<i32>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub unix_millis_request_time: Option<i32>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub priority: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub labels: Vec<String>,
}

impl TaskParams {
    pub fn start_time(&self) -> Option<i32> {
        self.unix_millis_earliest_start_time.clone()
    }

    pub fn start_time_mut(&mut self) -> &mut Option<i32> {
        &mut self.unix_millis_earliest_start_time
    }

    pub fn request_time(&self) -> Option<i32> {
        self.unix_millis_request_time.clone()
    }

    pub fn request_time_mut(&mut self) -> &mut Option<i32> {
        &mut self.unix_millis_request_time
    }

    pub fn priority(&self) -> Option<serde_json::Value> {
        self.priority.clone()
    }

    pub fn priority_mut(&mut self) -> &mut Option<serde_json::Value> {
        &mut self.priority
    }

    pub fn labels(&self) -> Vec<String> {
        self.labels.clone()
    }

    pub fn labels_mut(&mut self) -> &mut Vec<String> {
        &mut self.labels
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TaskRequest {
    pub category: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub description: serde_json::Value,
    #[serde(default, skip_serializing_if = "is_default")]
    pub description_display: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub requester: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub fleet_name: Option<String>,
}

impl Default for TaskRequest {
    fn default() -> Self {
        TaskRequest {
            category: "".to_string(),
            description: serde_json::Value::Null,
            description_display: None,
            requester: None,
            fleet_name: None,
        }
    }
}

impl TaskRequest {
    pub fn is_valid(&self) -> bool {
        if self.category.is_empty() {
            return false;
        }
        true
    }

    pub fn category(&self) -> String {
        self.category.clone()
    }

    pub fn category_mut(&mut self) -> &mut String {
        &mut self.category
    }

    pub fn description(&self) -> serde_json::Value {
        self.description.clone()
    }

    pub fn description_mut(&mut self) -> &mut serde_json::Value {
        &mut self.description
    }

    pub fn description_display(&self) -> Option<String> {
        self.description_display.clone()
    }

    pub fn description_display_mut(&mut self) -> &mut Option<String> {
        &mut self.description_display
    }

    pub fn requester(&self) -> Option<String> {
        self.requester.clone()
    }

    pub fn requester_mut(&mut self) -> &mut Option<String> {
        &mut self.requester
    }

    pub fn fleet_name(&self) -> Option<String> {
        self.fleet_name.clone()
    }

    pub fn fleet_name_mut(&mut self) -> &mut Option<String> {
        &mut self.fleet_name
    }
}

pub trait TaskRequestType {
    fn is_valid(&self) -> bool;

    fn request(&self) -> TaskRequest;

    fn request_mut(&mut self) -> &mut TaskRequest;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DispatchTaskRequest {
    pub request: TaskRequest,
}

impl TaskRequestType for DispatchTaskRequest {
    fn is_valid(&self) -> bool {
        self.request.is_valid()
    }

    fn request(&self) -> TaskRequest {
        self.request.clone()
    }

    fn request_mut(&mut self) -> &mut TaskRequest {
        &mut self.request
    }
}

impl DispatchTaskRequest {
    pub fn new(request: TaskRequest) -> Self {
        Self { request }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RobotTaskRequest {
    pub robot: String,
    pub fleet: String,
    pub request: TaskRequest,
}

impl TaskRequestType for RobotTaskRequest {
    fn is_valid(&self) -> bool {
        if self.fleet.is_empty() {
            return false;
        }
        if self.robot.is_empty() {
            return false;
        }
        self.request.is_valid()
    }

    fn request(&self) -> TaskRequest {
        self.request.clone()
    }

    fn request_mut(&mut self) -> &mut TaskRequest {
        &mut self.request
    }
}

impl RobotTaskRequest {
    pub fn new(robot: String, fleet: String, request: TaskRequest) -> Self {
        Self {
            robot,
            fleet,
            request,
        }
    }

    pub fn robot(&self) -> String {
        self.robot.clone()
    }

    pub fn robot_mut(&mut self) -> &mut String {
        &mut self.robot
    }

    pub fn fleet(&self) -> String {
        self.fleet.clone()
    }

    pub fn fleet_mut(&mut self) -> &mut String {
        &mut self.fleet
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Task {
    Dispatch(DispatchTaskRequest),
    Direct(RobotTaskRequest),
}

impl Default for Task {
    fn default() -> Self {
        Task::Dispatch(DispatchTaskRequest {
            request: TaskRequest::default(),
        })
    }
}

impl Task {
    pub fn is_valid(&self) -> bool {
        match self {
            Task::Dispatch(dispatch_task_request) => dispatch_task_request.is_valid(),
            Task::Direct(robot_task_request) => robot_task_request.is_valid(),
        }
    }

    pub fn is_dispatch(&self) -> bool {
        match self {
            Task::Dispatch(_) => true,
            _ => false,
        }
    }

    pub fn is_direct(&self) -> bool {
        match self {
            Task::Direct(_) => true,
            _ => false,
        }
    }

    pub fn request(&self) -> TaskRequest {
        match self {
            Task::Dispatch(dispatch_task_request) => dispatch_task_request.request(),
            Task::Direct(robot_task_request) => robot_task_request.request(),
        }
    }

    pub fn request_mut(&mut self) -> &mut TaskRequest {
        match self {
            Task::Dispatch(dispatch_task_request) => dispatch_task_request.request_mut(),
            Task::Direct(robot_task_request) => robot_task_request.request_mut(),
        }
    }

    pub fn robot(&self) -> String {
        match self {
            Task::Direct(robot_task_request) => robot_task_request.robot(),
            _ => "".to_string(),
        }
    }

    pub fn robot_mut(&mut self) -> Option<&mut String> {
        match self {
            Task::Direct(robot_task_request) => Some(robot_task_request.robot_mut()),
            _ => None,
        }
    }

    pub fn fleet(&self) -> String {
        match self {
            Task::Direct(robot_task_request) => robot_task_request.fleet(),
            _ => "".to_string(),
        }
    }

    pub fn fleet_mut(&mut self) -> Option<&mut String> {
        match self {
            Task::Direct(robot_task_request) => Some(robot_task_request.fleet_mut()),
            _ => None,
        }
    }
}

pub trait TaskKind {
    fn label() -> String;
}

// Supported Task kinds
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct GoToPlace {
    pub location: String,
}

impl Default for GoToPlace {
    fn default() -> Self {
        Self {
            location: String::new(),
        }
    }
}

impl fmt::Display for GoToPlace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.location)
    }
}

impl TaskKind for GoToPlace {
    fn label() -> String {
        "Go To Place".to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct WaitFor {
    pub duration: f32,
}

impl Default for WaitFor {
    fn default() -> Self {
        Self { duration: 0.0 }
    }
}

impl fmt::Display for WaitFor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} seconds", self.duration)
    }
}

impl TaskKind for WaitFor {
    fn label() -> String {
        "Wait For".to_string()
    }
}
