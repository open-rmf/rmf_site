use bevy::{
    asset::{AssetIo, AssetIoError, AssetPlugin, ChangeWatcher, FileType, Metadata},
    prelude::*,
    utils::{BoxedFuture, HashMap},
};
use dirs;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::{urdf_loader::UrdfPlugin, OSMTile};
use urdf_rs::utils::expand_package_path;

use rmf_site_format::AssetSource;

pub fn cache_path() -> PathBuf {
    let mut p = dirs::cache_dir().unwrap();
    p.push("open-robotics");
    p.push("rmf_site_editor");
    return p;
}

struct SiteAssetIo {
    pub default_io: Box<dyn AssetIo>,
    pub bundled_assets: HashMap<String, Vec<u8>>,
}

const FUEL_BASE_URI: &str = "https://fuel.gazebosim.org/1.0";
pub const MODEL_ENVIRONMENT_VARIABLE: &str = "GZ_SIM_RESOURCE_PATH";

pub static FUEL_API_KEY: Mutex<Option<String>> = Mutex::new(None);

#[derive(Deserialize)]
struct FuelErrorMsg {
    errcode: u32,
    msg: String,
}

impl SiteAssetIo {
    fn load_from_file(&self, path: PathBuf) -> Result<Vec<u8>, AssetIoError> {
        let mut bytes = Vec::new();
        match fs::File::open(&path) {
            Ok(mut file) => {
                file.read_to_end(&mut bytes)?;
            }
            Err(e) => {
                return if e.kind() == std::io::ErrorKind::NotFound {
                    Err(AssetIoError::NotFound(path))
                } else {
                    Err(e.into())
                }
            }
        }
        Ok(bytes)
    }

    fn fetch_asset<'a>(
        &'a self,
        remote_url: String,
        asset_name: String,
    ) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            let mut req = surf::get(remote_url.clone());
            match FUEL_API_KEY.lock() {
                Ok(key) => {
                    if let Some(key) = key.clone() {
                        req = req.header("Private-token", key);
                    }
                }
                Err(poisoned_key) => {
                    // Reset the key to None
                    *poisoned_key.into_inner() = None;
                    return Err(AssetIoError::Io(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Lock poisoning detected when reading fuel API key, please set it again."),
                    )));
                }
            }
            let bytes = req.recv_bytes().await.map_err(|e| {
                AssetIoError::Io(io::Error::new(io::ErrorKind::Other, e.to_string()))
            })?;

            match serde_json::from_slice::<FuelErrorMsg>(&bytes) {
                Ok(error) => {
                    return Err(AssetIoError::Io(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!(
                            "Failed to fetch asset from fuel {} [errcode {}]: {}",
                            remote_url, error.errcode, error.msg,
                        ),
                    )));
                }
                Err(_) => {
                    // This is actually the happy path. When a GET from fuel was
                    // successful, it will not return a JSON that can be
                    // interpreted as a FuelErrorMsg, so our attempt to parse an
                    // error message will fail.
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                self.save_to_cache(&asset_name, &bytes);
            }
            Ok(bytes)
        })
    }

    fn get_path_from_env(&self) -> Result<PathBuf, env::VarError> {
        let var = env::var(MODEL_ENVIRONMENT_VARIABLE)?;
        let path = PathBuf::from(var);
        // TODO wrap error to be more explicative
        match path.exists() {
            true => Ok(path),
            false => Err(env::VarError::NotPresent),
        }
    }

    fn save_to_cache(&self, name: &String, bytes: &[u8]) {
        let mut asset_path = cache_path();
        asset_path.push(PathBuf::from(&name));
        fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
        if bytes.len() > 0 {
            fs::write(asset_path, bytes).expect("unable to write to file");
        }
    }

    fn generate_remote_asset_url(&self, name: &String) -> Result<String, AssetIoError> {
        // Expected format: OrgName/ModelName/FileName.ext
        // We may need to be a bit magical here because some assets
        // are found in Fuel and others are not.
        let binding = name.clone();
        let mut tokens = binding.split("/");
        let org_name = match tokens.next() {
            Some(token) => token,
            None => {
                return Err(AssetIoError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unable to parse into org/model names: {name}"),
                )));
            }
        };
        let model_name = match tokens.next() {
            Some(token) => token,
            None => {
                return Err(AssetIoError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unable to parse into org/model names: {name}"),
                )));
            }
        };
        // TODO(luca) migrate to split.remainder once
        // https://github.com/rust-lang/rust/issues/77998 is stabilized
        let binding = tokens.fold(String::new(), |prefix, path| prefix + "/" + path);
        if binding.len() < 2 {
            return Err(AssetIoError::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("File name not found for: {name}"),
            )));
        }
        let filename = binding.split_at(1).1;
        let uri = format!(
            "{0}/{1}/models/{2}/tip/files/{3}",
            FUEL_BASE_URI, org_name, model_name, filename
        );
        return Ok(uri);
    }

    fn add_bundled_assets(&mut self) {
        self.bundled_assets.insert(
            "textures/select.png".to_owned(),
            include_bytes!("../../assets/textures/select.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/selected.png".to_owned(),
            include_bytes!("../../assets/textures/selected.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/trash.png".to_owned(),
            include_bytes!("../../assets/textures/trash.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/merge.png".to_owned(),
            include_bytes!("../../assets/textures/merge.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/confirm.png".to_owned(),
            include_bytes!("../../assets/textures/confirm.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/add.png".to_owned(),
            include_bytes!("../../assets/textures/add.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/reject.png".to_owned(),
            include_bytes!("../../assets/textures/reject.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/search.png".to_owned(),
            include_bytes!("../../assets/textures/search.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/empty.png".to_owned(),
            include_bytes!("../../assets/textures/empty.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/alignment.png".to_owned(),
            include_bytes!("../../assets/textures/alignment.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/edit.png".to_owned(),
            include_bytes!("../../assets/textures/edit.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/exit.png".to_owned(),
            include_bytes!("../../assets/textures/exit.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/up.png".to_owned(),
            include_bytes!("../../assets/textures/up.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/down.png".to_owned(),
            include_bytes!("../../assets/textures/down.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/to_top.png".to_owned(),
            include_bytes!("../../assets/textures/to_top.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/to_bottom.png".to_owned(),
            include_bytes!("../../assets/textures/to_bottom.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/opaque.png".to_owned(),
            include_bytes!("../../assets/textures/opaque.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/alpha.png".to_owned(),
            include_bytes!("../../assets/textures/alpha.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/hidden.png".to_owned(),
            include_bytes!("../../assets/textures/hidden.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/global.png".to_owned(),
            include_bytes!("../../assets/textures/global.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/hide.png".to_owned(),
            include_bytes!("../../assets/textures/hide.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/battery.png".to_owned(),
            include_bytes!("../../assets/textures/battery.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/parking.png".to_owned(),
            include_bytes!("../../assets/textures/parking.png").to_vec(),
        );
        self.bundled_assets.insert(
            "textures/stopwatch.png".to_owned(),
            include_bytes!("../../assets/textures/stopwatch.png").to_vec(),
        );
    }
}

impl AssetIo for SiteAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        let asset_source = AssetSource::from(path);
        match asset_source {
            AssetSource::Remote(asset_name) => {
                let remote_url: String = match self.generate_remote_asset_url(&asset_name) {
                    Ok(uri) => uri,
                    Err(e) => return Box::pin(async move { Err(e) }),
                };

                // Try local cache first
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut asset_path = cache_path();
                    asset_path.push(PathBuf::from(&asset_name));
                    if asset_path.exists() {
                        return Box::pin(async move { self.load_from_file(asset_path) });
                    }
                }

                // Get from remote server
                self.fetch_asset(remote_url, asset_name)
            }
            AssetSource::Local(filename) => Box::pin(async move {
                let full_path = PathBuf::from(filename);
                self.load_from_file(full_path)
            }),
            AssetSource::Bundled(filename) => {
                if self.bundled_assets.contains_key(&filename) {
                    return Box::pin(async move { Ok(self.bundled_assets[&filename].clone()) });
                } else {
                    return Box::pin(async move {
                        Err(AssetIoError::Io(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Bundled asset not found: {filename}"),
                        )))
                    });
                }
            }
            AssetSource::Package(_) => Box::pin(async move {
                // Split into package and path
                let path = (*expand_package_path(&String::from(&asset_source), None)).to_owned();
                self.load_from_file(PathBuf::from(path))
            }),
            AssetSource::Search(asset_name) => {
                // Order should be:
                // Relative to the building.yaml location, TODO, relative paths are tricky
                // Relative to some paths read from an environment variable (.. need to check what gz uses for models)
                // Relative to a cache directory
                // Attempt to fetch from the server and save it to the cache directory

                // TODO checking whether it's an sdf folder or a obj file
                if let Ok(mut path) = self.get_path_from_env() {
                    // Check if file exists
                    path.push(&asset_name);
                    if path.exists() {
                        return Box::pin(async move { self.load_from_file(path) });
                    }
                }

                // Try local cache
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut asset_path = cache_path();
                    asset_path.push(PathBuf::from(&asset_name));
                    if asset_path.exists() {
                        return Box::pin(async move { self.load_from_file(asset_path) });
                    }
                }

                let remote_url = match self.generate_remote_asset_url(&asset_name) {
                    Ok(uri) => uri,
                    Err(e) => return Box::pin(async move { Err(e) }),
                };

                // It cannot be found locally, so let's try to fetch it from the
                // remote server
                self.fetch_asset(remote_url, asset_name)
            }

            AssetSource::OSMTile {
                zoom,
                latitude,
                longitude,
            } => {
                return Box::pin(async move {
                    let tile = OSMTile::from_latlon(zoom, latitude, longitude);
                    tile.get_map_image().await.map_err(|e| {
                        AssetIoError::Io(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Unable to load map: {e}"),
                        ))
                    })
                });
            }
        }
    }

    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError> {
        if path.starts_with("rmf-site://") {
            return Ok(Metadata::new(FileType::File));
        } else {
            return self.default_io.get_metadata(path);
        }
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        self.default_io.read_directory(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        #[cfg(target_arch = "wasm32")]
        return false;

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.is_dir(path)
    }

    fn watch_path_for_changes(
        &self,
        to_watch: &Path,
        to_reload: Option<PathBuf>,
    ) -> Result<(), AssetIoError> {
        #[cfg(target_arch = "wasm32")]
        return Ok(());

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.watch_path_for_changes(to_watch, to_reload)
    }

    fn watch_for_changes(&self, configuration: &ChangeWatcher) -> Result<(), AssetIoError> {
        #[cfg(target_arch = "wasm32")]
        return Ok(());

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.watch_for_changes(configuration)
    }
}

/// A plugin used to execute the override of the asset io
pub struct SiteAssetIoPlugin;

impl Plugin for SiteAssetIoPlugin {
    fn build(&self, app: &mut App) {
        let mut asset_io = {
            let default_io = AssetPlugin::default().create_platform_default_asset_io();
            SiteAssetIo {
                default_io,
                bundled_assets: HashMap::new(),
            }
        };
        asset_io.add_bundled_assets();

        // the asset server is constructed and added the resource manager
        app.insert_resource(AssetServer::new(asset_io))
            .add_plugin(bevy_stl::StlPlugin)
            .add_plugin(bevy_obj::ObjPlugin)
            .add_plugin(UrdfPlugin);
    }
}
