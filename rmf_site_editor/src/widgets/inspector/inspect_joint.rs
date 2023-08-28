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

use crate::{
    site::{Change, Dependents, FrameMarker, JointAxis, JointLimit, JointType, SiteID},
    widgets::{inspector::SelectionWidget, AppEvents},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, Ui};

#[derive(SystemParam)]
pub struct InspectJointParams<'w, 's> {
    pub joints: Query<
        'w,
        's,
        (
            &'static Parent,
            &'static Dependents,
            &'static JointType,
            Option<&'static JointLimit>,
            Option<&'static JointAxis>,
        ),
    >,
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub frames: Query<'w, 's, (), With<FrameMarker>>,
}

pub struct InspectJointWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub joint: Entity,
    pub params: &'a InspectJointParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectJointWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        joint: Entity,
        params: &'a InspectJointParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            joint,
            params,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let Ok((parent, deps, joint_type, joint_limit, joint_axis)) = self.params.joints.get(self.joint) else {
            return;
        };

        let mut new_joint_type = joint_type.clone();

        ui.label("Parent frame");
        SelectionWidget::new(
            **parent,
            self.params.site_id.get(**parent).ok().cloned(),
            self.params.icons.as_ref(),
            self.events,
        )
        .show(ui);

        if let Some(frame_dep) = deps.iter().find(|d| self.params.frames.get(**d).is_ok()) {
            ui.label("Child frame");
            SelectionWidget::new(
                *frame_dep,
                self.params.site_id.get(*frame_dep).ok().cloned(),
                self.params.icons.as_ref(),
                self.events,
            )
            .show(ui);
        }

        ui.horizontal(|ui| {
            ui.label("Joint Type");
            ComboBox::from_id_source("Joint Type")
                .selected_text(new_joint_type.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        JointType::Fixed,
                        JointType::Revolute,
                        JointType::Prismatic,
                        JointType::Continuous,
                    ] {
                        ui.selectable_value(&mut new_joint_type, *variant, variant.label());
                    }
                });
        });

        if new_joint_type != *joint_type {
            self.events
                .change_more
                .joint_type
                .send(Change::new(new_joint_type, self.joint));
        }
        // TODO(luca) add joint limit and joint axis inspectors
    }
}
