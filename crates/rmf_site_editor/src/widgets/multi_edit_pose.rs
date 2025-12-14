use crate::{site::Change, widgets::prelude::*};
use bevy::math::Vec3A;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, Grid, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{Angle, Pose, Rotation};

use smallvec::SmallVec;

#[derive(SystemParam)]
pub struct MultiEditPoseWidget<'w, 's> {
    commands: Commands<'w, 's>,
    poses: Query<'w, 's, &'static Pose>,
}

impl<'w, 's> WidgetSystem<SmallVec<[Entity; 16]>, ()> for MultiEditPoseWidget<'w, 's> {
    fn show(
        instances: SmallVec<[Entity; 16]>,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(instances, ui);
    }
}

impl<'w, 's> MultiEditPoseWidget<'w, 's> {
    pub fn show_widget(&mut self, instances: SmallVec<[Entity; 16]>, ui: &mut Ui) {
        ui.label("Modify poses");

        // Calculate centroid of selected instances
        // let mut centroid = Pose::default();
        let mut centroid = Vec3A::ZERO;
        let mut orientation = Angle::Deg(0.0);

        for instance in &instances {
            if let Ok(pose) = self.poses.get(*instance) {
                centroid += Vec3A::from(pose.trans);
            } else {
                return;
            }
        }
        centroid /= instances.len() as f32;
        // Use the orientation of the first instance for the centroid pose
        if let Ok(pose) = self.poses.get(instances[0]) {
            orientation = pose.rot.yaw();
        }

        let frame = Pose {
            trans: centroid.into(),
            rot: Rotation::Yaw(orientation),
        };

        if let Some(new_frame) = InspectMultiPoseComponent::new(&frame).show(ui) {
            let trans_offset = Vec3A::from(new_frame.trans) - centroid;
            let rot_offset = new_frame.rot.yaw() - orientation;

            // trigger change of pose to all selected instances
            for instance in instances {
                if let Ok(pose) = self.poses.get(instance.clone()) {
                    let new_trans: [f32; 3] = (Vec3A::from(pose.trans) + trans_offset).into();
                    let mut new_yaw = rot_offset + pose.rot.yaw();
                    new_yaw.wrap_to_pi();

                    let new_pose = Pose {
                        trans: new_trans,
                        rot: Rotation::Yaw(new_yaw),
                    };

                    self.commands.trigger(Change::new(new_pose, instance));
                }
            }
        }
    }
}

pub struct InspectMultiPoseComponent<'a> {
    pub pose: &'a Pose,
}

impl<'a> InspectMultiPoseComponent<'a> {
    pub fn new(pose: &'a Pose) -> Self {
        Self { pose }
    }

    pub fn show(self, ui: &mut Ui) -> Option<Pose> {
        let mut new_pose = self.pose.clone();

        Grid::new("inspect_multi_pose_translation").show(ui, |ui| {
            ui.label("x");
            ui.label("y");
            ui.label("z");
            ui.end_row();

            ui.add(DragValue::new(&mut new_pose.trans[0]).speed(0.01));
            ui.add(DragValue::new(&mut new_pose.trans[1]).speed(0.01));
            ui.add(DragValue::new(&mut new_pose.trans[2]).speed(0.01));
            ui.end_row();
        });
        ui.add_space(5.0);

        if new_pose != *self.pose {
            return Some(new_pose);
        }

        None
    }
}
