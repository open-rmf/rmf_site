use crate::ros2_package_path::{Ros2PackagePath, Ros2PackagePathError};
use std::error::Error;
use std::io::Error as IoError;

pub fn get_mesh_files(
    urdf_robot: &urdf_rs::Robot,
) -> Result<Vec<Ros2PackagePath>, Ros2PackagePathError> {
    // TODO: handle the case where a mesh is only in a collision and not a visual
    urdf_robot
        .links
        .iter()
        .flat_map(|link| link.visual.iter().map(|visual| &visual.geometry))
        .filter_map(|geometry| match geometry {
            urdf_rs::Geometry::Mesh { filename, .. } => Some(filename),
            _ => None,
        })
        .map(|filename| Ros2PackagePath::from_string(filename))
        .collect()
}

pub fn replace_mesh_file_paths(
    urdf_robot: &mut urdf_rs::Robot,
    new_package_name: &str,
    mesh_directory_name: &str,
) -> Result<(), Box<dyn Error>> {
    if mesh_directory_name.starts_with('/') {
        IoError::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Mesh directory must not start with a slash, but is: {}",
                mesh_directory_name
            ),
        );
    }
    if mesh_directory_name.ends_with('/') {
        IoError::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Mesh directory must not end with a slash, but is: {}",
                mesh_directory_name
            ),
        );
    }
    for link in urdf_robot.links.iter_mut() {
        for visual in link.visual.iter_mut() {
            if let urdf_rs::Geometry::Mesh { filename, .. } = &mut visual.geometry {
                update_mesh_path(filename, mesh_directory_name, new_package_name)?;
            }
        }
        for collision in link.collision.iter_mut() {
            if let urdf_rs::Geometry::Mesh { filename, .. } = &mut collision.geometry {
                update_mesh_path(filename, mesh_directory_name, new_package_name)?;
            }
        }
    }
    Ok(())
}

fn update_mesh_path(
    filename: &mut String,
    mesh_directory_name: &str,
    new_package_name: &str,
) -> Result<(), Box<dyn Error>> {
    let current_ros_path = Ros2PackagePath::from_string(filename)?;
    let new_relative_path = format!(
        "{}/{}",
        mesh_directory_name,
        current_ros_path.get_file_name()
    );
    let new_ros_path = Ros2PackagePath::new(new_package_name.to_string(), new_relative_path);
    filename.clear();
    filename.push_str(&new_ros_path.get_path());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGE_NAME: &str = "test_package";
    const RELATIVE_MESH_PATH_1: &str = "meshes/mesh1.stl";
    const RELATIVE_MESH_PATH_2: &str = "meshes/mesh2.stl";

    fn get_urdf() -> urdf_rs::Robot {
        let urdf_string = format!(
            r#"<?xml version="1.0"?>
            <robot name="test_robot">

                <link name="base_link">
                    <visual>
                        <geometry>
                            <cylinder length="0.6" radius="0.2" />
                        </geometry>
                        <material name="blue" />
                    </visual>
                    <collision>
                        <geometry>
                            <cylinder length="0.6" radius="0.2" />
                        </geometry>
                    </collision>
                    <inertial>
                        <mass value="10" />
                        <inertia ixx="1e-3" ixy="0.0" ixz="0.0" iyy="1e-3" iyz="0.0" izz="1e-3" />
                    </inertial>
                </link>

                <joint name="gripper_extension" type="prismatic">
                    <parent link="base_link" />
                    <child link="gripper_pole" />
                    <limit effort="1000.0" lower="-0.38" upper="0" velocity="0.5" />
                    <origin rpy="0 0 0" xyz="0.19 0 0.2" />
                </joint>

                <link name="gripper_pole">
                    <visual>
                        <geometry>
                            <cylinder length="0.2" radius="0.01" />
                        </geometry>
                        <origin rpy="0 1.57075 0 " xyz="0.1 0 0" />
                    </visual>
                    <collision>
                        <geometry>
                            <cylinder length="0.2" radius="0.01" />
                        </geometry>
                        <origin rpy="0 1.57075 0 " xyz="0.1 0 0" />
                    </collision>
                    <inertial>
                        <mass value="0.05" />
                        <inertia ixx="1e-3" ixy="0.0" ixz="0.0" iyy="1e-3" iyz="0.0" izz="1e-3" />
                    </inertial>
                </link>

                <joint name="right_gripper_joint" type="revolute">
                    <axis xyz="0 0 -1" />
                    <limit effort="1000.0" lower="0.0" upper="0.548" velocity="0.5" />
                    <origin rpy="0 0 0" xyz="0.2 -0.01 0" />
                    <parent link="gripper_pole" />
                    <child link="right_gripper" />
                </joint>

                <link name="right_gripper">
                    <visual>
                        <origin rpy="-3.1415 0 0" xyz="0 0 0" />
                        <geometry>
                            <mesh filename="package://{package_name}/{relative_mesh_path_1}" />
                        </geometry>
                    </visual>
                    <collision>
                        <geometry>
                            <mesh filename="package://{package_name}/{relative_mesh_path_1}" />
                        </geometry>
                        <origin rpy="-3.1415 0 0" xyz="0 0 0" />
                    </collision>
                    <inertial>
                        <mass value="0.05" />
                        <inertia ixx="1e-3" ixy="0.0" ixz="0.0" iyy="1e-3" iyz="0.0" izz="1e-3" />
                    </inertial>
                </link>

                <joint name="right_tip_joint" type="fixed">
                    <parent link="right_gripper" />
                    <child link="right_tip" />
                </joint>

                <link name="right_tip">
                    <visual>
                        <origin rpy="-3.1415 0 0" xyz="0.09137 0.00495 0" />
                        <geometry>
                            <mesh filename="package://{package_name}/{relative_mesh_path_2}" />
                        </geometry>
                    </visual>
                    <collision>
                        <geometry>
                            <mesh filename="package://{package_name}/{relative_mesh_path_2}" />
                        </geometry>
                        <origin rpy="-3.1415 0 0" xyz="0.09137 0.00495 0" />
                    </collision>
                    <inertial>
                        <mass value="0.05" />
                        <inertia ixx="1e-3" ixy="0.0" ixz="0.0" iyy="1e-3" iyz="0.0" izz="1e-3" />
                    </inertial>
                </link>

            </robot>
        "#,
            package_name = PACKAGE_NAME,
            relative_mesh_path_1 = RELATIVE_MESH_PATH_1,
            relative_mesh_path_2 = RELATIVE_MESH_PATH_2
        );
        urdf_rs::read_from_string(&urdf_string).expect("URDF should be valid")
    }

    #[test]
    fn test_get_mesh_files() {
        let urdf_robot = get_urdf();
        let mesh_files = get_mesh_files(&urdf_robot).expect("Should get mesh files");
        assert_eq!(mesh_files.len(), 2);

        assert_eq!(mesh_files[0].package_name, PACKAGE_NAME);
        assert_eq!(mesh_files[0].relative_path, RELATIVE_MESH_PATH_1);

        assert_eq!(mesh_files[1].package_name, PACKAGE_NAME);
        assert_eq!(mesh_files[1].relative_path, RELATIVE_MESH_PATH_2);
    }

    #[test]
    fn test_replace_mesh_file_paths() {
        let mut urdf_robot = get_urdf();
        let new_package_name = "new_package";
        let new_mesh_directory = "new_directory/meshes";
        replace_mesh_file_paths(&mut urdf_robot, new_package_name, new_mesh_directory)
            .expect("Should replace mesh file paths");
        let mesh_files = get_mesh_files(&urdf_robot).expect("Should get mesh files");
        for mesh_file in mesh_files {
            assert_eq!(mesh_file.package_name, new_package_name);
            assert!(mesh_file.relative_path.starts_with(new_mesh_directory));
        }
    }
}
