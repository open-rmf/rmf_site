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

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum PortingError {
    #[error("a non-existent vertex [{0}] was referenced")]
    InvalidVertex(usize),
    #[error("a non-existent level {0} was referenced")]
    InvalidLevelName(String),
    #[error("wrong number [{door_count}] of lift cabin doors for lift [{lift}]; must be no greater than 1")]
    InvalidLiftCabinDoors{lift: String, door_count: usize},
    #[error("wrong number [{door_count}] of level doors for lift [{lift}] on level [{level}]")]
    InvalidLiftLevelDoorCount{lift: String, level: String, door_count: usize},
    #[error("unable to find a door named {door} on level {level} requested by lift {lift}")]
    InvalidLiftLevelDoorName{lift: String, level: String, door: String},
    #[error("the data contained a known type which has been deprecated: {0}")]
    DeprecatedType(String),
    #[error("the data contained an unknown/invalid type: {0}")]
    InvalidType(String),
}

pub type Result<T> = std::result::Result<T, PortingError>;
