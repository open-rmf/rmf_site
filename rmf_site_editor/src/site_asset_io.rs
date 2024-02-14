use bevy::asset::io::AssetSource as BevyAssetSource;
use bevy::{
    asset::embedded_asset,
    asset::io::{
        AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
        VecReader,
    },
    prelude::*,
    utils::{BoxedFuture, HashMap},
};
use dirs;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::marker::Sync;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::OSMTile;
use urdf_rs::utils::expand_package_path;

use rmf_site_format::AssetSource;

pub fn cache_path() -> PathBuf {
    let mut p = dirs::cache_dir().unwrap();
    p.push("open-robotics");
    p.push("rmf_site_editor");
    return p;
}

const FUEL_BASE_URI: &str = "https://fuel.gazebosim.org/1.0";
pub const MODEL_ENVIRONMENT_VARIABLE: &str = "GZ_SIM_RESOURCE_PATH";

pub static FUEL_API_KEY: Mutex<Option<String>> = Mutex::new(None);

#[derive(Deserialize)]
struct FuelErrorMsg {
    errcode: u32,
    msg: String,
}

fn load_from_file<'a>(path: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
    match fs::read(&path) {
        Ok(bytes) => Ok(Box::new(VecReader::new(bytes))),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(AssetReaderError::NotFound(path))
            } else {
                Err(e.into())
            }
        }
    }
}

fn generate_remote_asset_url(name: &str) -> Result<String, AssetReaderError> {
    // Expected format: OrgName/ModelName/FileName.ext
    // We may need to be a bit magical here because some assets
    // are found in Fuel and others are not.
    let binding = name.to_owned();
    let mut tokens = binding.split("/");
    let org_name = match tokens.next() {
        Some(token) => token,
        None => {
            return Err(AssetReaderError::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("Unable to parse into org/model names: {name}"),
            )));
        }
    };
    let model_name = match tokens.next() {
        Some(token) => token,
        None => {
            return Err(AssetReaderError::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("Unable to parse into org/model names: {name}"),
            )));
        }
    };
    // TODO(luca) migrate to split.remainder once
    // https://github.com/rust-lang/rust/issues/77998 is stabilized
    let binding = tokens.fold(String::new(), |prefix, path| prefix + "/" + path);
    if binding.len() < 2 {
        return Err(AssetReaderError::Io(io::Error::new(
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

async fn fetch_asset<'a>(
    remote_url: String,
    asset_name: String,
) -> Result<Box<Reader<'a>>, AssetReaderError> {
    let mut req = ehttp::Request::get(remote_url.clone());
    match FUEL_API_KEY.lock() {
        Ok(key) => {
            if let Some(key) = key.clone() {
                req.headers.headers.push(("Private-token".to_owned(), key));
            }
        }
        Err(poisoned_key) => {
            // Reset the key to None
            *poisoned_key.into_inner() = None;
            return Err(AssetReaderError::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("Lock poisoning detected when reading fuel API key, please set it again."),
            )));
        }
    }
    let bytes = ehttp::fetch_async(req)
        .await
        .map_err(|e| AssetReaderError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?
        .bytes;

    match serde_json::from_slice::<FuelErrorMsg>(&bytes) {
        Ok(error) => {
            return Err(AssetReaderError::Io(io::Error::new(
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
        save_to_cache(&asset_name, &bytes);
    }
    Ok(Box::new(VecReader::new(bytes)))
}

fn get_path_from_env() -> Result<PathBuf, env::VarError> {
    let var = env::var(MODEL_ENVIRONMENT_VARIABLE)?;
    let path = PathBuf::from(var);
    // TODO wrap error to be more explicative
    match path.exists() {
        true => Ok(path),
        false => Err(env::VarError::NotPresent),
    }
}

fn save_to_cache(name: &str, bytes: &[u8]) {
    let mut asset_path = cache_path();
    asset_path.push(PathBuf::from(name));
    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
    if bytes.len() > 0 {
        fs::write(asset_path, bytes).expect("unable to write to file");
    }
}

pub struct SiteAssetReader<F>
where
    F: Fn(&Path) -> BoxedFuture<'_, Result<Box<Reader<'_>>, AssetReaderError>> + Sync + 'static,
{
    pub default_reader: Box<dyn AssetReader>,
    pub reader: F,
}

impl<F> SiteAssetReader<F>
where
    F: Fn(&Path) -> BoxedFuture<'_, Result<Box<Reader<'_>>, AssetReaderError>> + Sync + 'static,
{
    pub fn new(reader: F) -> Self {
        Self {
            default_reader: (AssetSourceBuilder::platform_default("assets", None)
                .reader
                .unwrap())(),
            reader,
        }
    }
}

impl<F> AssetReader for SiteAssetReader<F>
where
    F: Fn(&Path) -> BoxedFuture<'_, Result<Box<Reader<'_>>, AssetReaderError>>
        + Send
        + Sync
        + 'static,
{
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        (self.reader)(path)
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        self.default_reader.read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        self.default_reader.read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        self.default_reader.is_directory(path)
    }
}

/// A plugin used to execute the override of the asset io
pub struct SiteAssetIoPlugin;

impl Plugin for SiteAssetIoPlugin {
    fn build(&self, app: &mut App) {
        // the asset server is constructed and added the resource manager
        app.register_asset_source(
            "search",
            //AssetSourceBuilder::platform_default("assets", None),
            BevyAssetSource::build().with_reader(|| {
                Box::new(SiteAssetReader::new(|path: &Path| {
                    // Order should be:
                    // Relative to the building.yaml location
                    // Relative to the MODEL_ENVIRONMENT_VARIABLE path
                    // Relative to a cache directory
                    // Attempt to fetch from the server and save it to the cache directory

                    let asset_name = path.to_str().unwrap().to_owned();
                    if let Ok(mut path) = get_path_from_env() {
                        // Check if file exists
                        path.push(&asset_name);
                        if path.exists() {
                            return Box::pin(async move { load_from_file(path) });
                        }
                    }

                    // Try local cache
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut asset_path = cache_path();
                        asset_path.push(PathBuf::from(&asset_name));
                        if asset_path.exists() {
                            return Box::pin(async move { load_from_file(asset_path) });
                        }
                    }

                    let remote_url = match generate_remote_asset_url(&asset_name) {
                        Ok(uri) => uri,
                        Err(e) => return Box::pin(async move { Err(e) }),
                    };

                    // It cannot be found locally, so let's try to fetch it from the
                    // remote server
                    Box::pin(async move { fetch_asset(remote_url, asset_name).await })
                }))
            }),
        )
        .register_asset_source(
            "rmf-server",
            BevyAssetSource::build().with_reader(|| {
                Box::new(SiteAssetReader::new(|path: &Path| {
                    let asset_name = path.to_str().unwrap().to_owned();
                    let remote_url: String = match generate_remote_asset_url(&asset_name) {
                        Ok(uri) => uri,
                        Err(e) => return Box::pin(async move { Err(e) }),
                    };

                    // Try local cache first
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut asset_path = cache_path();
                        asset_path.push(path);
                        if asset_path.exists() {
                            return Box::pin(async move { load_from_file(asset_path) });
                        }
                    }
                    Box::pin(async move { fetch_asset(remote_url, asset_name).await })
                }))
            }),
        )
        .register_asset_source(
            "package",
            BevyAssetSource::build().with_reader(|| {
                Box::new(SiteAssetReader::new(|path: &Path| {
                    let path = (*expand_package_path(path.to_str().unwrap(), None)).to_owned();
                    Box::pin(async move { load_from_file(path.into()) })
                }))
            }),
        )
        .register_asset_source(
            "file",
            BevyAssetSource::build().with_reader(|| {
                Box::new(SiteAssetReader::new(|path: &Path| {
                    Box::pin(async move { load_from_file(path.into()) })
                }))
            }),
        )
        .register_asset_source(
            "osm-tile",
            BevyAssetSource::build().with_reader(|| {
                Box::new(SiteAssetReader::new(|path: &Path| {
                    Box::pin(async move {
                        let tile =
                            OSMTile::try_from(path.to_path_buf()).map_err(std::io::Error::other)?;
                        tile.get_map_image().await
                    })
                }))
            }),
        );
    }
}
