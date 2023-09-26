use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum Ros2PackagePathError {
    InvalidPrefix(String),
    MissingSeparator(String),
}

impl fmt::Display for Ros2PackagePathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ros2PackagePathError::InvalidPrefix(s) => {
                write!(f, "Package path '{}' does not start with 'package://'", s)
            }
            Ros2PackagePathError::MissingSeparator(s) => {
                write!(f, "Package path '{}' does not contain a '/'", s)
            }
        }
    }
}

impl Error for Ros2PackagePathError {}

#[derive(Debug)]
pub struct Ros2PackagePath {
    pub package_name: String,
    pub relative_path: String,
}

static PACKAGE_PREFIX: &str = "package://";

impl Ros2PackagePath {
    pub fn new(package_name: String, relative_path: String) -> Self {
        Self {
            package_name,
            relative_path,
        }
    }
    pub fn get_path(&self) -> String {
        format!("package://{}/{}", self.package_name, self.relative_path)
    }
    pub fn get_file_name(&self) -> String {
        let mut path_parts: Vec<&str> = self.relative_path.split('/').collect();
        path_parts.pop().unwrap().to_string()
    }
    pub fn from_string(package_path: &str) -> Result<Ros2PackagePath, Ros2PackagePathError> {
        if !package_path.starts_with(PACKAGE_PREFIX) {
            return Err(Ros2PackagePathError::InvalidPrefix(
                package_path.to_string(),
            ));
        }
        let package_path = package_path.trim_start_matches(PACKAGE_PREFIX);
        let mut package_path_parts: Vec<&str> = package_path.split('/').collect();
        if package_path_parts.len() < 2 {
            return Err(Ros2PackagePathError::MissingSeparator(
                package_path.to_string(),
            ));
        }
        let package_name = package_path_parts.remove(0).to_string();
        let relative_path = package_path_parts.join("/");
        Ok(Ros2PackagePath::new(package_name, relative_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_a_valid_string() {
        let inputs = vec![
            "package://my_package_name/my_relative_path",
            "package://my_package_name/my_relative_path/with/multiple/levels",
        ];
        for input in inputs {
            let ros2_package_path = Ros2PackagePath::from_string(input);
            assert!(ros2_package_path.is_ok());

            let ros2_package_path = ros2_package_path.unwrap();
            assert_eq!(ros2_package_path.package_name, "my_package_name");
            let expected_relative_path = input.trim_start_matches("package://my_package_name/");
            assert_eq!(ros2_package_path.relative_path, expected_relative_path);

            let output = ros2_package_path.get_path();
            assert_eq!(output, input);
        }
    }

    #[test]
    fn test_from_string_invalid_prefix() {
        let inputs = vec![
            "my_package_name/my_relative_path",
            "://my_package_name/my_relative_path",
            "invalid://my_package_name/my_relative_path",
            "invalid://my_package_name/my_relative_path/with/multiple/levels",
        ];
        for input in inputs {
            let ros2_package_path = Ros2PackagePath::from_string(input);
            assert!(ros2_package_path.is_err());
            assert!(matches!(
                ros2_package_path.unwrap_err(),
                Ros2PackagePathError::InvalidPrefix(_)
            ));
        }
    }

    #[test]
    fn test_from_string_missing_separator() {
        let input = "package://my_package_name";
        let ros2_package_path = Ros2PackagePath::from_string(input);
        assert!(ros2_package_path.is_err());
        assert!(matches!(
            ros2_package_path.unwrap_err(),
            Ros2PackagePathError::MissingSeparator(_)
        ));
    }

    #[test]
    fn test_get_file_name() {
        let file_name = "my_file_name.dae".to_string();
        let inputs = vec![
            format!("package://my_package_name/{}", file_name),
            format!(
                "package://my_package_name/my_relative_path/with/multiple/levels/{}",
                file_name
            ),
        ];
        for input in inputs {
            let ros2_package_path = Ros2PackagePath::from_string(&input).unwrap();
            assert_eq!(ros2_package_path.get_file_name(), file_name);
        }
    }
}
