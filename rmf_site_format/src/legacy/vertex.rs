use super::rbmf::*;
use crate::{
    is_default, AssetSource, AssociatedGraphs, ConstraintDependents, DifferentialDrive, Instance,
    InstanceBundle, IsStatic, Location, LocationTags, MobileRobot, MobileRobotKinematics, Model,
    ModelMarker, Models, NameInSite, Pose, Scale,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct VertexProperties {
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_charger: RbmfBool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_parking_spot: RbmfBool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_holding_point: RbmfBool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub spawn_robot_name: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub spawn_robot_type: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub dropoff_ingestor: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub pickup_dispenser: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub dock_name: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub lift_cabin: RbmfString,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Vertex(
    pub f64,
    pub f64,
    pub f64,
    pub String,
    #[serde(default)] pub VertexProperties,
);

impl Vertex {
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.0, self.1)
    }

    pub fn make_location(&self, anchor: u32) -> Option<Location<u32>> {
        let mut tags = LocationTags::default();
        let me = &self.4;
        if me.is_charger.1 {
            tags.charger = Some(String::new());
        }

        if me.is_parking_spot.1 {
            tags.parking = Some(String::new());
        }

        if me.is_holding_point.1 {
            tags.holding = Some(String::new());
        }

        let name = if self.3.is_empty() {
            None
        } else {
            Some(self.3.clone())
        };

        if tags.is_empty() && name.is_none() {
            return None;
        } else {
            return Some(Location {
                anchor: anchor.into(),
                tags,
                name: NameInSite(name.unwrap_or("<Unnamed>".to_string())),
                graphs: AssociatedGraphs::All,
            });
        }
    }

    pub fn make_instance(
        &self,
        new_site_id: &mut std::ops::RangeFrom<u32>,
        model_source_map: &mut HashMap<String, u32>,
        models: &mut Models,
        anchor: u32,
    ) -> Option<Instance> {
        let me = &self.4;
        if !me.spawn_robot_name.is_empty() && !me.spawn_robot_type.is_empty() {
            if let Some(model_id) = model_source_map.get(&me.spawn_robot_type.1) {
                return Some(Instance {
                    parent: anchor,
                    model: *model_id,
                    bundle: InstanceBundle {
                        name: NameInSite(me.spawn_robot_name.1.clone()),
                        pose: Pose::default(),
                    },
                });
            }

            // Create a new model for this asset source that we have not seen
            // before.
            let model_id = new_site_id.next().unwrap();
            let mobile_robot = MobileRobot {
                model_name: NameInSite(me.spawn_robot_type.1.clone()),
                source: AssetSource::Search(
                    "OpenRobotics/".to_owned() + &me.spawn_robot_type.1.clone(),
                ),
                scale: Scale::default(),
                kinematics: MobileRobotKinematics::DifferentialDrive(DifferentialDrive {
                    translational_speed: 0.5,
                    rotational_speed: 0.6,
                    bidirectional: false,
                }),
            };

            models.mobile_robots.insert(model_id, mobile_robot);
            model_source_map.insert(me.spawn_robot_type.1.clone(), model_id);
            return Some(Instance {
                parent: anchor,
                model: model_id,
                bundle: InstanceBundle {
                    name: NameInSite(me.spawn_robot_name.1.clone()),
                    pose: Pose::default(),
                },
            });
        }

        return None;
    }
}
