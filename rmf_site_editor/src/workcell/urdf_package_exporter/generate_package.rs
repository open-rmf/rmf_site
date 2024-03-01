use crate::site_asset_io::cache_path;
use crate::workcell::urdf_package_exporter::template::PackageContext;
use rmf_site_format::{AssetSource, Geometry, Workcell};
use std::error::Error;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::path::{Path, PathBuf};
use tera::Tera;

pub fn generate_package(
    workcell: Workcell,
    package_context: PackageContext,
    output_directory_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let new_package_name = &package_context.project_name;

    let output_package_path = output_directory_path.join(new_package_name);
    std::fs::create_dir_all(&output_package_path)?;

    // Create the package
    write_urdf_and_copy_mesh_files(workcell, &new_package_name, &output_package_path)?;
    generate_templates(package_context, &output_package_path)?;

    Ok(())
}

fn write_urdf_and_copy_mesh_files(
    mut workcell: Workcell,
    new_package_name: &str,
    output_package_path: &Path,
) -> Result<(), Box<dyn Error>> {
    convert_and_copy_meshes(&mut workcell, new_package_name, output_package_path)?;

    let urdf_robot = workcell.to_urdf()?;
    let urdf_directory_path = output_package_path.join("urdf");
    std::fs::create_dir_all(&urdf_directory_path)?;
    let urdf_file_path = urdf_directory_path.join("robot.urdf");
    let urdf_string = urdf_rs::write_to_string(&urdf_robot)?;
    std::fs::write(urdf_file_path, urdf_string)?;

    Ok(())
}

fn convert_and_copy_meshes(
    workcell: &mut Workcell,
    package_name: &str,
    output_package_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let meshes_directory_path = output_package_path.join("meshes");
    std::fs::create_dir_all(&meshes_directory_path)?;
    for (_, model) in &mut workcell
        .visuals
        .iter_mut()
        .chain(workcell.collisions.iter_mut())
    {
        if let Geometry::Mesh {
            source: asset_source,
            ..
        } = &mut model.bundle.geometry
        {
            let path = get_path_to_asset_file(&asset_source)?;

            let file_name = path
                .file_name()
                .ok_or(IoError::new(
                    IoErrorKind::InvalidInput,
                    "Unable to get file name from path",
                ))?
                .to_str()
                .ok_or(IoError::new(
                    IoErrorKind::InvalidInput,
                    "Unable to convert file name to str",
                ))?;

            std::fs::copy(&path, &meshes_directory_path.join(&file_name))?;
            let package_path = format!("{}/meshes/{}", package_name, file_name);
            *asset_source = AssetSource::Package(package_path);
        }
    }
    Ok(())
}

fn get_path_to_asset_file(asset_source: &AssetSource) -> Result<PathBuf, Box<dyn Error>> {
    match asset_source {
        AssetSource::Package(_) => Ok(urdf_rs::utils::expand_package_path(
            &(String::from(asset_source)),
            None,
        )
        .to_string()
        .into()),
        AssetSource::Remote(asset_name) => {
            let mut asset_path = cache_path();
            asset_path.push(&asset_name);
            Ok(asset_path)
        },
        AssetSource::RCC(asset_name) => {
            let mut asset_path = cache_path();
            asset_path.push(&asset_name);
            Ok(asset_path)
        },
        AssetSource::Local(path) => Ok(path.into()),
        AssetSource::Search(_) | AssetSource::OSMTile { .. } | AssetSource::Bundled(_) => {
            Err(IoError::new(
                IoErrorKind::Unsupported,
                "Not a supported asset type for exporting a workcell to a package",
            ))?
        }
    }
}

fn generate_templates(
    package_context: PackageContext,
    package_directory: &Path,
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
    tera.render_to("package.xml", &context, f)?;

    let f = std::fs::File::create(package_directory.join("CMakeLists.txt"))?;
    tera.render_to("CMakeLists.txt", &context, f)?;

    let rviz_directory = package_directory.join("rviz");
    std::fs::create_dir_all(&rviz_directory)?;
    let f = std::fs::File::create(rviz_directory.join("urdf.rviz"))?;
    tera.render_to("urdf.rviz", &context, f)?;

    let launch_directory = package_directory.join("launch");
    std::fs::create_dir_all(&launch_directory)?;
    let f = std::fs::File::create(launch_directory.join("display.launch.py"))?;
    tera.render_to("display.launch.py", &context, f)?;

    Ok(())
}
