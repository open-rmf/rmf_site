use std::error::Error;
use std::io;
use std::process::Command;
use std::str;

pub fn get_pkg_prefix(package_name: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("ros2")
        .arg("pkg")
        .arg("prefix")
        .arg(package_name)
        .output()?;

    if output.status.success() {
        Ok(str::from_utf8(&output.stdout)?.trim().to_string())
    } else {
        let stderr = str::from_utf8(&output.stderr)?;
        Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to get package prefix for package '{}': {}",
                package_name, stderr
            ),
        )))
    }
}

// These tests require ROS 2 to be sourced.
#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn test_valid_package_name() {
        let package_name = "rclcpp";
        let package_prefix = super::get_pkg_prefix(package_name).expect("Should return a path");
        assert!(!package_prefix.is_empty());
    }

    #[test]
    #[ignore]
    fn test_invalid_package_name() {
        let package_name = "invalid_package_name";
        let result = super::get_pkg_prefix(package_name);
        assert!(result.is_err());
    }
}
