use serde::Serialize;
use std::error::Error;
use std::path::PathBuf;
use tera::Tera;

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct Person {
    pub name: String,
    pub email: String,
}

pub struct Template {
    pub name: String,
    pub path: String,
    pub output_path: PathBuf,
}

pub fn populate_and_save_templates(
    templates: Vec<Template>,
    context: &PackageContext,
) -> Result<(), Box<dyn Error>> {
    let context = tera::Context::from_serialize(context)?;
    let mut tera = Tera::default();
    for template in templates.iter() {
        let content = std::fs::read_to_string(&template.path)?;
        tera.add_raw_template(&template.name, &content)?;
        let rendered = tera.render(&template.name, &context)?;
        std::fs::write(&template.output_path, rendered)?;
    }
    Ok(())
}
