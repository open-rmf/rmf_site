use rmf_site_format::{Workcell, AssetSource, Geometry};
use std::path::{PathBuf, Path};
use std::process::Output;
use dirs;
use std::error::Error;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use crate::template;

pub fn generate_package_2(
    workcell: &Workcell,
    package_context: &template::PackageContext,
    output_directory: &String,
) -> Result<(), Box<dyn Error>> {

    let output_directory = "output".to_string();
    let new_package_name = "test_package".to_string();

    let mesh_directory_name = "meshes".to_string();
    let launch_directory_name = "launch".to_string();
    let urdf_directory_name = "urdf".to_string();
    let rviz_directory_name = "rviz".to_string();

    // Create paths
    let output_directory_path = std::path::Path::new(&output_directory);
    let output_package_path = output_directory_path.join(&new_package_name);
    let meshes_directory_path = output_package_path.join(&mesh_directory_name);
    let launch_directory_path = output_package_path.join(&launch_directory_name);
    let urdf_directory_path = output_package_path.join(&urdf_directory_name);
    let rviz_directory_path = output_package_path.join(&rviz_directory_name);

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
    write_urdf_and_copy_mesh_files(&workcell, &mesh_directory_name, &meshes_directory_path, &new_package_name, &urdf_directory_path);
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

fn convert_to_package_paths(workcell: &Workcell, package_name: &str, mesh_directory_name: &str) -> Result<Workcell, Box<dyn Error>> {
    let mut workcell = workcell.clone();
    workcell.visuals.iter_mut().chain(workcell.collisions.iter_mut()).try_for_each(|(id, visual)| {
        if let Geometry::Mesh { source: asset_source, scale } = &mut visual.bundle.geometry {
            let path = get_path_to_asset_file(asset_source)?;

            let file_name = PathBuf::from(&path).file_name().ok_or(
                IoError::new(IoErrorKind::InvalidInput, "Unable to get file name from path")
            )?.to_str().ok_or(
                IoError::new(IoErrorKind::InvalidInput, "Unable to convert file name to str")
            )?.to_owned();

            let package_path = format!("package://{}/{}/{}", package_name, mesh_directory_name, file_name);
            *asset_source = AssetSource::Package(package_path);
        }
        Result::<(), Box<dyn Error>>::Ok(())
    })?;
    Ok(workcell)
}

fn get_mesh_paths(workcell: &Workcell) -> Result<Vec<String>, Box<dyn Error>> {
    let paths = workcell.visuals.iter().chain(workcell.collisions.iter()).filter_map(|(id, visual)| {
        get_path_if_mesh_geometry(&visual.bundle.geometry).ok()?
    }).collect();
    Ok(paths)
}

fn get_path_if_mesh_geometry(geometry: &Geometry) -> Result<Option<String>, Box<dyn Error>> {
    if let Geometry::Mesh { source: asset_source, scale } = geometry {
        Ok(Some(get_path_to_asset_file(asset_source)?))
    } else {
        Ok(None)
    }
}


fn copy_files(paths: &Vec<String>, output_directory: &Path) -> Result<(), Box<dyn Error>> {
    for path in paths.iter() {
        let file_name = PathBuf::from(path).file_name().ok_or(
            IoError::new(IoErrorKind::InvalidInput, "Unable to get file name from path")
        )?.to_owned();
        let new_path = PathBuf::from(output_directory).join(file_name);
        match std::fs::copy(path, &new_path) {
            Ok(_) => {},
            Err(e) => {
                println!("Error copying file '{}' to '{}': {}", path, new_path.display(), e);
            }
        }
    }
    Ok(())
}


// TODO(anyone) remove duplication with rmf_site_editor
fn cache_path() -> PathBuf {
    let mut p = dirs::cache_dir().unwrap();
    p.push("open-robotics");
    p.push("rmf_site_editor");
    return p;
}

fn get_path_to_asset_file(asset_source: &AssetSource) -> Result<String, Box<dyn Error>> {
    if let AssetSource::Package(_) = asset_source {
        let path = String::from(asset_source);
        Ok((*urdf_rs::utils::expand_package_path(&path, None)).to_owned())
    } else if let AssetSource::Remote(asset_name) = asset_source {
        let mut asset_path = cache_path();
        asset_path.push(PathBuf::from(&asset_name));
        Ok(asset_path.to_str().ok_or(IoError::new(IoErrorKind::InvalidInput, "Unable to convert asset_path to str"))?.to_string())
    } else if let AssetSource::Local(path) = asset_source {
        Ok(path.clone())
    } else {
        Err(IoError::new(IoErrorKind::Unsupported, "Not a supported asset type for exporting a workcell to a package"))?
    }
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


