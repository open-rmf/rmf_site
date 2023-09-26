use package_exporter::{generate_package,PackageContext, Person};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <urdf_file> <output_directory>", args[0]);
    }
    let urdf_path = &args[1];
    let output_directory = &args[2];

    let package_context = PackageContext {
        project_name: "output_package".to_string(),
        project_version: "1.0.0".to_string(),
        project_description: "A generated package from an rmf_site workcell".to_string(),
        license: "MIT".to_string(),
        maintainers: vec![Person {
            name: "Audrow Nash".to_string(),
            email: "audrow@intrinsic.ai".to_string(),
        }],
        dependencies: vec![],
        fixed_frame: "base_link".to_string(),
        urdf_file_name: "robot.urdf".to_string(),
    };

    let urdf_robot = urdf_rs::read_file(urdf_path).expect("Should be able to read URDF file");
    generate_package(&urdf_robot, &package_context, output_directory).unwrap();
    println!("Wrote package to: {}", output_directory);
}
