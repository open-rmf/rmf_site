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

use bevy_egui::egui::Ui;
use rmf_site_format::IsPrimary;

pub struct InspectIsPrimary {
    pub is_primary: IsPrimary,
}

impl InspectIsPrimary {
    #[allow(dead_code)]
    pub fn new(is_primary: &IsPrimary) -> Self {
        Self {
            is_primary: *is_primary,
        }
    }

    #[allow(dead_code)]
    pub fn show(self, ui: &mut Ui) -> Option<IsPrimary> {
        let mut new_is_primary = self.is_primary;
        ui.checkbox(&mut new_is_primary.0, "Primary")
            .on_hover_text("Primary drawings will be used as a reference against other drawings when optimizing transforms");

        if new_is_primary != self.is_primary {
            return Some(new_is_primary);
        }

        None
    }
}
