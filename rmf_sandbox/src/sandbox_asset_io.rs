use bevy::{
    asset::{AssetIo, AssetIoError},
    prelude::*,
    utils::BoxedFuture,
};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use dirs::cache_dir;

struct SandboxAssetIo {
    pub default_io: Box<dyn AssetIo>,
}

impl AssetIo for SandboxAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        info!("load_path({:?})", path);
        if path.starts_with("sandbox://") {
            let cache_path = cache_dir().unwrap().to_str().to_string() + "open-robotics/sandbox";
            let cached_path = cache_path + path.to_str().unwrap().strip_prefix("sandbox://").unwrap(); // //path.to_str().unwrap();
            Box::pin(async move {
                //let full_path = self.root_path.join(cached_path);
                let mut bytes = Vec::new();
                match File::open(&cached_path) {
                    Ok(mut file) => {
                        file.read_to_end(&mut bytes)?;
                    }
                    Err(e) => {
                        return if e.kind() == std::io::ErrorKind::NotFound {
                            Err(AssetIoError::NotFound(PathBuf::from(cached_path)))
                        } else {
                            Err(e.into())
                        }
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
        self.default_io.is_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        info!("watch_path_for_changes({:?})", path);
        self.default_io.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        info!("watch_for_changes()");
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
