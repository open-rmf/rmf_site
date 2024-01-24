use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct PackageContext {
    pub project_name: String,
    pub project_description: String,
    pub project_version: String,
    pub license: String,
    pub maintainers: Vec<Person>,
    pub dependencies: Vec<String>,
    pub fixed_frame: String,
    pub urdf_file_name: String,
}

#[derive(Debug, Serialize)]
pub struct Person {
    pub name: String,
    pub email: String,
}

#[derive(Debug)]
pub struct Template {
    pub name: String,
    pub path: String,
    pub output_path: PathBuf,
}
