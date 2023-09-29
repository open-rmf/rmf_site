use crate::template;
use crate::urdf;

use std::error::Error;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::path::Path;

pub fn generate_package(
    urdf_robot: &urdf_rs::Robot,
    package_context: &template::PackageContext,
    output_directory: &String,
) -> Result<(), Box<dyn Error>> {
    let mut urdf_robot = urdf_robot.clone();
    let new_package_name = &package_context.project_name;

    let mesh_directory_name = "meshes".to_string();
    let launch_directory_name = "launch".to_string();
    let urdf_directory_name = "urdf".to_string();
    let rviz_directory_name = "rviz".to_string();

    // Create paths
    let output_directory_path = std::path::Path::new(output_directory);
    let output_package_path = output_directory_path.join(new_package_name);
    let meshes_directory_path = output_package_path.join(&mesh_directory_name);
    let launch_directory_path = output_package_path.join(&launch_directory_name);
    let urdf_directory_path = output_package_path.join(&urdf_directory_name);
    let rviz_directory_path = output_package_path.join(&rviz_directory_name);

    // Create directories
    if output_package_path.exists() {
        std::fs::remove_dir_all(&output_package_path)?;
    }
    for directory_path in [
        &output_package_path,
        &meshes_directory_path,
        &launch_directory_path,
        &urdf_directory_path,
        &rviz_directory_path,
    ] {
        std::fs::create_dir_all(directory_path)?;
    }

    write_urdf_and_copy_mesh_files(
        &mut urdf_robot,
        &mesh_directory_name,
        &meshes_directory_path,
        new_package_name,
        &urdf_directory_path,
    )?;

    generate_templates(
        package_context,
        &output_package_path,
        &launch_directory_path,
        &rviz_directory_path,
    )?;

    Ok(())
}

fn write_urdf_and_copy_mesh_files(
    urdf_robot: &mut urdf_rs::Robot,
    mesh_directory_name: &str,
    meshes_directory_path: &std::path::Path,
    new_package_name: &str,
    urdf_directory_path: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    // Current mesh files
    let mesh_files = urdf::get_mesh_files(urdf_robot)?;
    // Copy mesh files to new directory
    for mesh_file in mesh_files.iter() {
        let mesh_file_path = (*urdf_rs::utils::expand_package_path(mesh_file.get_path().as_str(), None)).to_owned();
        let output_mesh_file_path = meshes_directory_path.join(&mesh_file.get_file_name());
        std::fs::copy(&mesh_file_path, &output_mesh_file_path)?;
    }

    // Update mesh files
    urdf::replace_mesh_file_paths(urdf_robot, new_package_name, mesh_directory_name)?;
    // Write URDF file
    let urdf_file_path = urdf_directory_path.join("robot.urdf");
    let urdf_string = urdf_rs::write_to_string(urdf_robot)?;
    std::fs::write(urdf_file_path, urdf_string)?;

    Ok(())
}

fn generate_templates(
    package_context: &template::PackageContext,
    package_directory: &std::path::Path,
    launch_directory: &std::path::Path,
    rviz_directory: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let directory = Path::new(file!()).parent().ok_or_else(|| {
        IoError::new(
            IoErrorKind::Other,
            format!("Could not get directory of {}", file!()),
        )
    })?.to_str().ok_or_else(|| {
        IoError::new(
            IoErrorKind::Other,
            format!("Could not convert directory of {} to string", file!()),
        )
    })?;
    let templates = vec![
        template::Template {
            name: "package.xml".to_string(),
            path: format!("{}/templates/package.xml.j2", directory),
            output_path: package_directory.join("package.xml"),
        },
        template::Template {
            name: "CMakeLists.txt".to_string(),
            path: format!("{}/templates/CMakeLists.txt.j2", directory),
            output_path: package_directory.join("CMakeLists.txt"),
        },
        template::Template {
            name: "urdf.rviz".to_string(),
            path: format!("{}/templates/urdf.rviz.j2", directory),
            output_path: rviz_directory.join("urdf.rviz"),
        },
        template::Template {
            name: "display.launch.py".to_string(),
            path: format!("{}/templates/display.launch.py.j2", directory),
            output_path: launch_directory.join("display.launch.py"),
        },
    ];
    template::populate_and_save_templates(templates, package_context)?;
    Ok(())
}

