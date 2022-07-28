use bevy::{
    asset::{AssetIo, AssetIoError, Metadata, FileType},
    prelude::*,
    utils::BoxedFuture,
};
use dirs;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub fn cache_path() -> PathBuf {
    let mut p = dirs::cache_dir().unwrap();
    p.push("open-robotics");
    p.push("rmf_sandbox");
    return p;
}

struct SandboxAssetIo {
    pub default_io: Box<dyn AssetIo>,
}

const SANDBOX_MODELS_URI: &str = "https://models.sandbox.open-rmf.org/models/";

impl AssetIo for SandboxAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        if path.starts_with("sandbox://") {
            let without_prefix = path.to_str().unwrap().strip_prefix("sandbox://").unwrap();
            let uri = String::from(SANDBOX_MODELS_URI) + without_prefix;

            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut asset_path = cache_path();
                asset_path.push(PathBuf::from(without_prefix));
                if asset_path.exists() {
                    return Box::pin(async move {
                        let mut bytes = Vec::new();
                        match fs::File::open(&asset_path) {
                            Ok(mut file) => {
                                file.read_to_end(&mut bytes)?;
                            }
                            Err(e) => {
                                return if e.kind() == std::io::ErrorKind::NotFound {
                                    Err(AssetIoError::NotFound(asset_path))
                                } else {
                                    Err(e.into())
                                }
                            }
                        }
                        Ok(bytes)
                    });
                }
            }

            Box::pin(async move {
                let bytes = surf::get(uri).recv_bytes().await.map_err(|e| {
                    AssetIoError::Io(io::Error::new(io::ErrorKind::Other, e.to_string()))
                })?;

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut asset_path = cache_path();
                    asset_path.push(PathBuf::from(without_prefix));
                    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
                    if bytes.len() > 0 {
                        fs::write(asset_path, &bytes).expect("unable to write to file");
                    }
                }
                Ok(bytes)
            })
        } else {
            self.default_io.load_path(path)
        }
    }

    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError> {
        if path.starts_with("sandbox://") {
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

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        #[cfg(target_arch = "wasm32")]
        return Ok(());

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        #[cfg(target_arch = "wasm32")]
        return Ok(());

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.watch_for_changes()
    }
}

/// A plugin used to execute the override of the asset io
pub struct SandboxAssetIoPlugin;

impl Plugin for SandboxAssetIoPlugin {
    fn build(&self, app: &mut App) {
        let asset_io = {
            let default_io = bevy::asset::create_platform_default_asset_io(app);
            SandboxAssetIo {
                default_io,
            }
        };

        // the asset server is constructed and added the resource manager
        app.insert_resource(AssetServer::new(asset_io));
    }
}
