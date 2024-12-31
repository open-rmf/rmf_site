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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Model {
    /// Name of the model
    pub name: NameInSite,
    /// Where the model should be loaded from
    pub source: AssetSource,
    /// Pose of the model relative to the level it is on.
    pub pose: Pose,
    #[serde(default, skip_serializing_if = "is_default")]
    /// Whether this model should be able to move in simulation
    pub is_static: IsStatic,
    /// Scale to be applied to the model
    #[serde(default, skip_serializing_if = "is_default")]
    pub scale: Scale,
    /// Only relevant for bevy
    #[serde(skip)]
    pub marker: ModelMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct ModelMarker;

impl Default for Model {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_string()),
            source: AssetSource::default(),
            pose: Pose::default(),
            is_static: IsStatic(false),
            scale: Scale::default(),
            marker: ModelMarker,
        }
    }
}

/// Defines a property in a model description, that will be added to all instances
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct ModelProperty<T: Default + Clone>(pub T);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OptionalModelProperty<T: RefTrait> {
    DifferentialDrive(DifferentialDrive),
    MobileRobotMarker(MobileRobotMarker),
    Tasks(Tasks<T>),
}

impl<T: RefTrait> Default for OptionalModelProperty<T> {
    fn default() -> Self {
        OptionalModelProperty::DifferentialDrive(DifferentialDrive::default())
    }
}

impl<T: RefTrait> OptionalModelProperty<T> {
    pub fn convert<U: RefTrait>(
        &self,
        id_map: &HashMap<T, U>,
    ) -> Result<OptionalModelProperty<U>, T> {
        let result = match self {
            Self::DifferentialDrive(diff_drive) => {
                OptionalModelProperty::DifferentialDrive(diff_drive.clone())
            }
            Self::MobileRobotMarker(mobile_marker) => {
                OptionalModelProperty::MobileRobotMarker(mobile_marker.clone())
            }
            Self::Tasks(tasks) => OptionalModelProperty::Tasks(tasks.convert(id_map)?),
        };
        Ok(result)
    }
}

/// Defines a property in a model description, that will be added to all instances
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct OptionalModelProperties<T: RefTrait>(pub Vec<OptionalModelProperty<T>>);

impl<T: RefTrait> Default for OptionalModelProperties<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: RefTrait> OptionalModelProperties<T> {
    pub fn convert<U: RefTrait>(
        &self,
        id_map: &HashMap<T, U>,
    ) -> Result<OptionalModelProperties<U>, T> {
        self.0.iter().try_fold(
            OptionalModelProperties::default(),
            |mut optional_properties, property| {
                optional_properties.0.push(property.convert(id_map)?);
                Ok(optional_properties)
            },
        )
    }
}

/// Bundle with all required components for a valid model description
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct ModelDescriptionBundle<T: RefTrait> {
    pub name: NameInSite,
    pub source: ModelProperty<AssetSource>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_static: ModelProperty<IsStatic>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub scale: ModelProperty<Scale>,
    #[serde(skip)]
    pub group: Group,
    #[serde(skip)]
    pub marker: ModelMarker,
    pub optional_properties: OptionalModelProperties<T>,
}

impl<T: RefTrait> Default for ModelDescriptionBundle<T> {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_string()),
            source: ModelProperty(AssetSource::default()),
            is_static: ModelProperty(IsStatic::default()),
            scale: ModelProperty(Scale::default()),
            group: Group,
            marker: ModelMarker,
            optional_properties: OptionalModelProperties::default(),
        }
    }
}

/// Bundle with all required components for a valid model instance
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct ModelInstance<T: RefTrait> {
    pub name: NameInSite,
    pub pose: Pose,
    pub parent: SiteParent<T>,
    pub description: Affiliation<T>,
    #[serde(skip)]
    pub marker: ModelMarker,
    #[serde(skip)]
    pub instance_marker: InstanceMarker,
    pub optional_properties: OptionalModelProperties<T>,
}

impl<T: RefTrait> Default for ModelInstance<T> {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_string()),
            pose: Pose::default(),
            parent: SiteParent::default(),
            description: Affiliation::default(),
            marker: ModelMarker,
            instance_marker: InstanceMarker,
            optional_properties: OptionalModelProperties::default(),
        }
    }
}

impl<T: RefTrait> ModelInstance<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ModelInstance<U>, T> {
        Ok(ModelInstance {
            name: self.name.clone(),
            pose: self.pose.clone(),
            parent: self.parent.convert(id_map)?,
            description: self.description.convert(id_map)?,
            optional_properties: self.optional_properties.convert(id_map)?,
            ..Default::default()
        })
    }
}
