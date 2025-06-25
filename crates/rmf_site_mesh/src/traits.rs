use bevy_mod_outline::{
    GenerateOutlineNormalsError, GenerateOutlineNormalsSettings, OutlineMeshExt,
};
use bevy_render::mesh::Mesh;

pub trait WithOutlineMeshExt: Sized {
    fn with_generated_outline_normals(self) -> Result<Self, GenerateOutlineNormalsError>;
}

impl WithOutlineMeshExt for Mesh {
    fn with_generated_outline_normals(mut self) -> Result<Self, GenerateOutlineNormalsError> {
        self.generate_outline_normals(&GenerateOutlineNormalsSettings::default())?;
        Ok(self)
    }
}
