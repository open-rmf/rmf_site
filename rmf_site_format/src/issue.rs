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

use crate::RefTrait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone, Ord, PartialOrd)]
pub struct IssueKey<T: RefTrait> {
    /// Denotes which entities this issue affects
    pub entities: BTreeSet<T>,
    /// Uuid of the type of issue
    pub kind: Uuid,
}
