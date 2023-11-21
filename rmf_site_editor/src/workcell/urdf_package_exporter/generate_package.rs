use crate::site_asset_io::cache_path;
use crate::workcell::urdf_package_exporter::template;
use rmf_site_format::{AssetSource, Geometry, Workcell};
use std::error::Error;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::path::{Path, PathBuf};
use tera::Tera;

pub fn generate_package(
    workcell: &Workcell,
    package_context: &template::PackageContext,
    output_directory: &String,
) -> Result<(), Box<dyn Error>> {
    let new_package_name = &package_context.project_name;

    let mesh_directory_name = "meshes";

    // Create paths
    let output_directory_path = std::path::Path::new(&output_directory);
    let output_package_path = output_directory_path.join(new_package_name);
    let meshes_directory_path = output_package_path.join(mesh_directory_name);
    let launch_directory_path = output_package_path.join("launch");
    let urdf_directory_path = output_package_path.join("urdf");
    let rviz_directory_path = output_package_path.join("rviz");

    // Create directories
    if output_package_path.exists() {
        std::fs::remove_dir_all(&output_directory_path)?;
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

    // Create the package
    write_urdf_and_copy_mesh_files(
        &workcell,
        &mesh_directory_name,
        &meshes_directory_path,
        &new_package_name,
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
    workcell: &Workcell,
    mesh_directory_name: &str,
    meshes_directory_path: &std::path::Path,
    new_package_name: &str,
    urdf_directory_path: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let asset_paths = get_mesh_paths(&workcell)?;
    copy_files(&asset_paths, &meshes_directory_path)?;

    let new_workcell = convert_to_package_paths(&workcell, new_package_name, mesh_directory_name)?;

    let urdf_robot = new_workcell.to_urdf()?;
    let urdf_file_path = urdf_directory_path.join("robot.urdf");
    let urdf_string = urdf_rs::write_to_string(&urdf_robot)?;
    std::fs::write(urdf_file_path, urdf_string)?;

    Ok(())
}

fn convert_to_package_paths(
    workcell: &Workcell,
    package_name: &str,
    mesh_directory_name: &str,
) -> Result<Workcell, Box<dyn Error>> {
    let mut workcell = workcell.clone();
    workcell
        .visuals
        .iter_mut()
        .chain(workcell.collisions.iter_mut())
        .try_for_each(|(id, visual)| {
            if let Geometry::Mesh {
                source: asset_source,
                scale: _,
            } = &mut visual.bundle.geometry
            {
                let path = get_path_to_asset_file(asset_source)?;

                let file_name = PathBuf::from(&path)
                    .file_name()
                    .ok_or(IoError::new(
                        IoErrorKind::InvalidInput,
                        "Unable to get file name from path",
                    ))?
                    .to_str()
                    .ok_or(IoError::new(
                        IoErrorKind::InvalidInput,
                        "Unable to convert file name to str",
                    ))?
                    .to_owned();

                let package_path =
                    format!("{}/{}/{}", package_name, mesh_directory_name, file_name);
                *asset_source = AssetSource::Package(package_path);
            }
            Result::<(), Box<dyn Error>>::Ok(())
        })?;
    Ok(workcell)
}

fn get_mesh_paths(workcell: &Workcell) -> Result<Vec<String>, Box<dyn Error>> {
    let paths = workcell
        .visuals
        .iter()
        .chain(workcell.collisions.iter())
        .filter_map(|(id, visual)| get_path_if_mesh_geometry(&visual.bundle.geometry).ok()?)
        .collect();
    Ok(paths)
}

fn get_path_if_mesh_geometry(geometry: &Geometry) -> Result<Option<String>, Box<dyn Error>> {
    if let Geometry::Mesh {
        source: asset_source,
        scale,
    } = geometry
    {
        Ok(Some(get_path_to_asset_file(asset_source)?))
    } else {
        Ok(None)
    }
}

fn copy_files(paths: &Vec<String>, output_directory: &Path) -> Result<(), Box<dyn Error>> {
    for path in paths.iter() {
        let file_name = PathBuf::from(path)
            .file_name()
            .ok_or(IoError::new(
                IoErrorKind::InvalidInput,
                "Unable to get file name from path",
            ))?
            .to_owned();
        let new_path = PathBuf::from(output_directory).join(file_name);
        match std::fs::copy(path, &new_path) {
            Ok(_) => {}
            Err(e) => {
                println!(
                    "Error copying file '{}' to '{}': {}",
                    path,
                    new_path.display(),
                    e
                );
            }
        }
    }
    Ok(())
}

fn get_path_to_asset_file(asset_source: &AssetSource) -> Result<String, Box<dyn Error>> {
    match asset_source {
        AssetSource::Package(_) => Ok((*urdf_rs::utils::expand_package_path(
            &(String::from(asset_source)),
            None,
        ))
        .to_owned()),
        AssetSource::Remote(asset_name) => {
            let mut asset_path = cache_path();
            asset_path.push(PathBuf::from(&asset_name));
            Ok(asset_path
                .to_str()
                .ok_or(IoError::new(
                    IoErrorKind::InvalidInput,
                    "Unable to convert asset_path to str",
                ))?
                .to_string())
        }
        AssetSource::Local(path) => Ok(path.clone()),
        AssetSource::Search(_) | AssetSource::OSMTile { .. } | AssetSource::Bundled(_) => {
            Err(IoError::new(
                IoErrorKind::Unsupported,
                "Not a supported asset type for exporting a workcell to a package",
            ))?
        }
    }
}

fn generate_templates(
    package_context: &template::PackageContext,
    package_directory: &std::path::Path,
    launch_directory: &std::path::Path,
    rviz_directory: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let context = tera::Context::from_serialize(package_context)?;
    let mut tera = Tera::default();
    tera.add_raw_template("package.xml", include_str!("templates/package.xml.j2"))?;
    tera.add_raw_template(
        "CMakeLists.txt",
        include_str!("templates/CMakeLists.txt.j2"),
    )?;
    tera.add_raw_template("urdf.rviz", include_str!("templates/urdf.rviz.j2"))?;
    tera.add_raw_template(
        "display.launch.py",
        include_str!("templates/display.launch.py.j2"),
    )?;
    let f = std::fs::File::create(package_directory.join("package.xml"))?;
    let rendered = tera.render_to("package.xml", &context, f)?;
    let f = std::fs::File::create(package_directory.join("CMakeLists.txt"))?;
    let rendered = tera.render_to("CMakeLists.txt", &context, f)?;
    let f = std::fs::File::create(rviz_directory.join("urdf.rviz"))?;
    let rendered = tera.render_to("urdf.rviz", &context, f)?;
    let f = std::fs::File::create(launch_directory.join("display.launch.py"))?;
    let rendered = tera.render_to("display.launch.py", &context, f)?;
    Ok(())
}
