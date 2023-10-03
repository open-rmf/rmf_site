use rmf_site_format::{Workcell, AssetSource, Geometry};
use std::path::PathBuf;
use dirs;
use std::error::Error;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};


fn cache_path() -> PathBuf {
    let mut p = dirs::cache_dir().unwrap();
    p.push("open-robotics");
    p.push("rmf_site_editor");
    return p;
}

fn get_path_if_mesh_geometry(geometry: &Geometry) -> Result<Option<String>, Box<dyn Error>> {
    if let Geometry::Mesh { source: asset_source, scale } = geometry {
        Ok(Some(get_path_to_asset_file(asset_source)?))
    } else {
        Ok(None)
    }
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
        Err(IoError::new(IoErrorKind::Unsupported, ""))?
    }
}

pub fn generate_package_2() {
    let path_to_workcell = "/usr/local/google/home/audrow/Documents/nexus/rmf_site/test-local.workcell.json";
   let workcell_contents = std::fs::read_to_string(path_to_workcell).unwrap();
    let workcell = Workcell::from_str(&workcell_contents).unwrap();

    let visuals = workcell.visuals;
    let paths: Vec<_> = visuals.iter().filter_map(|(id, visual)| {
        get_path_if_mesh_geometry(&visual.bundle.geometry).ok()?
    }).collect();
    
    println!("Visuals");
    for path in paths.iter() {
        println!("{}", path);
    }

    let collisions = workcell.collisions;
    let paths: Vec<_> = collisions.iter().filter_map(|(id, visual)| {
        get_path_if_mesh_geometry(&visual.bundle.geometry).ok()?
    }).collect();
    
    println!("Collisions");
    for path in paths.iter() {
        println!("{}", path);
    }
}
