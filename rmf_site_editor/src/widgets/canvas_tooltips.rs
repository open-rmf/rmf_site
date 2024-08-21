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

use bevy::ecs::prelude::Resource;
use bevy_egui::egui::{self, Context};
use std::borrow::Cow;

/// This resource holds a list of tooltips that are emitted by 3D canvas interactions
/// and should be rendered near the cursor. Each tip will be given its own line in a
/// text block.
///
/// The `tips` field is cleared out on each iteration to prevent sticky tooltips.
#[derive(Resource, Default)]
pub struct CanvasTooltips {
    /// Push a `Cow::Borrowed("Your string literal here")` to push a `&'static str`,
    /// which is the most efficient way to produce tooltip text.
    ///
    /// If your text is not a string literal then push `Cow::Owned(your_string)`.
    tips: Vec<Cow<'static, str>>,
    /// We keep track of previous tooltips to keep order as consistent as possible
    /// to prevent flickering as it's possible for different systems to add in
    /// tooltips at different times.
    previous: Vec<Cow<'static, str>>,
}

impl CanvasTooltips {
    pub fn add(&mut self, tip: Cow<'static, str>) {
        if self.tips.contains(&tip) {
            return;
        }
        self.tips.push(tip);
    }
}

impl CanvasTooltips {
    pub fn render(&mut self, ctx: &Context) {
        if self.tips.is_empty() {
            self.previous.clear();
            return;
        }

        let mut decremented = 0;
        for (i, old_tip) in self.previous.iter().enumerate() {
            let i_actual = i - decremented;
            if let Some((j, _)) = self
                .tips
                .iter()
                .enumerate()
                .find(|(_, new_tip)| *old_tip == **new_tip)
            {
                if i_actual != j {
                    self.tips.swap(i_actual, j);
                }
            } else {
                decremented += 1;
            }
        }

        let text = self.tips.join("\n");

        egui::containers::popup::show_tooltip_text(ctx, "cursor_tooltip".into(), text);

        self.previous = self.tips.drain(..).collect();
    }
}
