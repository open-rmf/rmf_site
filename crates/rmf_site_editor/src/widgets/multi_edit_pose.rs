use crate::{
    site::Change,
    widgets::{inspector::InspectPoseComponent, prelude::*},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::Ui;
use rmf_site_egui::WidgetSystem;
use rmf_site_format::Pose;

#[derive(SystemParam)]
pub struct MultiEditPoseWidget<'w, 's> {
    commands: Commands<'w, 's>,
    poses: Query<'w, 's, &'static Pose>,
}

impl<'w, 's> WidgetSystem<Vec<Entity>, ()> for MultiEditPoseWidget<'w, 's> {
    fn show(instances: Vec<Entity>, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        params.show_widget(instances, ui);
    }
}

impl<'w, 's> MultiEditPoseWidget<'w, 's> {
    pub fn show_widget(&mut self, instances: Vec<Entity>, ui: &mut Ui) {
        ui.label("Modify multi-selection poses");

        // Calculate centroid of selected instances
        let mut centroid = Pose::default();
        for instance in &instances {
            if let Ok(pose) = self.poses.get(*instance) {
                centroid.trans[0] += pose.trans[0];
                centroid.trans[1] += pose.trans[1];
                centroid.trans[2] += pose.trans[2];
            } else {
                return;
            }
        }
        centroid.trans[0] /= instances.len() as f32;
        centroid.trans[1] /= instances.len() as f32;
        centroid.trans[2] /= instances.len() as f32;

        if let Some(new_centroid) = InspectPoseComponent::new(&centroid).show(ui) {
            let mut trans_offset: [f32; 3] = [0.0; 3];
            trans_offset[0] = new_centroid.trans[0] - centroid.trans[0];
            trans_offset[1] = new_centroid.trans[1] - centroid.trans[1];
            trans_offset[2] = new_centroid.trans[2] - centroid.trans[2];
            let yaw_offset = new_centroid.rot.yaw() - centroid.rot.yaw();
            info!("Yaw Offset: {:?}", yaw_offset);

            // trigger change of pose to all selected instances
            for instance in instances {
                if let Ok(pose) = self.poses.get(instance.clone()) {
                    let mut new_pose = *pose;
                    new_pose.trans[0] += trans_offset[0];
                    new_pose.trans[1] += trans_offset[1];
                    new_pose.trans[2] += trans_offset[2];

                    new_pose.rot.apply_yaw(yaw_offset);

                    self.commands.trigger(Change::new(new_pose, instance));
                }
            }
        }
    }
}
