use bevy::{
    asset::{AssetIo, AssetIoError},
    prelude::*,
    utils::BoxedFuture,
};
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use dirs;
use std::fs;
use std::io;

pub fn cache_path() -> PathBuf {
    let mut p = dirs::cache_dir().unwrap();
    p.push("open-robotics");
    p.push("rmf_sandbox");
    return p;
}

struct SandboxAssetIo {
    pub default_io: Box<dyn AssetIo>,
}

impl AssetIo for SandboxAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        info!("load_path({:?})", path);
        if path.starts_with("sandbox://") {
            let without_prefix = path
                .to_str()
                .unwrap()
                .strip_prefix("sandbox://")
                .unwrap();
            let uri = String::from("https://models.sandbox.open-rmf.org/models/")
                + without_prefix;

            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut asset_path = cache_path();
                asset_path.push(PathBuf::from(without_prefix));
                if asset_path.exists() {
                    println!("loading {} from cache", &asset_path.clone().into_os_string().into_string().unwrap());
                    return Box::pin(async move {
                        let mut bytes = Vec::new();
                        match File::open(&asset_path) {
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
                    })
                }
            }

            Box::pin(async move {
                info!("downloading from {}", &uri);
                let bytes = surf::get(uri)
                    .recv_bytes()
                    .await
                    .map_err(|e| AssetIoError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
                info!("received {} bytes", bytes.len());

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut asset_path = cache_path();
                    asset_path.push(PathBuf::from(without_prefix));
                    create_dir_all(asset_path.parent().unwrap()).unwrap();
                    println!("saving to {}", &asset_path.clone().into_os_string().into_string().unwrap());
                    if bytes.len() > 0 {
                        fs::write(asset_path, &bytes).expect("unable to write to file");
                    }
                }
                Ok(bytes)
            })
        }
        else {
            self.default_io.load_path(path)
        }
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        info!("read_directory({:?})", path);

        self.default_io.read_directory(path)
    }

    fn is_directory(&self, path: &Path) -> bool {
        info!("is_directory({:?})", path);

        #[cfg(target_arch = "wasm32")]
        return false;

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.is_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        info!("watch_path_for_changes({:?})", path);

        #[cfg(target_arch = "wasm32")]
        return Ok(());

        #[cfg(not(target_arch = "wasm32"))]
        self.default_io.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        info!("watch_for_changes()");

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
        // must get a hold of the task pool in order to create the asset server
        let task_pool = app.world.resource::<bevy::tasks::IoTaskPool>().0.clone();

        let asset_io = {
            let default_io = bevy::asset::create_platform_default_asset_io(app);
            SandboxAssetIo {
                default_io: default_io,
            }
        };

        // the asset server is constructed and added the resource manager
        app.insert_resource(AssetServer::new(asset_io, task_pool));
    }
}
