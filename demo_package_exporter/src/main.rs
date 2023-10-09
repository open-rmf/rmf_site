use rmf_site_format::Workcell;
use package_exporter::{generate_package,PackageContext, Person, generate_package_2};

fn generate_package_1() {
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
fn main() {
    let path_to_workcell = "/usr/local/google/home/audrow/Documents/nexus/rmf_site/test-local.workcell.json";
    let output_directory = "output".to_string();

    let workcell_contents = std::fs::read_to_string(path_to_workcell).unwrap();
    let workcell = Workcell::from_str(&workcell_contents).unwrap();

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
    generate_package_2(&workcell, &package_context, &output_directory);
}
