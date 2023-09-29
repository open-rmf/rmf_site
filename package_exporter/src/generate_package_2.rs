use rmf_site_format::Workcell;

pub fn generate_package_2() {
    let path_to_workcell = "/usr/local/google/home/audrow/Documents/nexus/rmf_site/test2.workcell.json";
    let workcell_contents = std::fs::read_to_string(path_to_workcell).unwrap();
    println!("{}", workcell_contents);
    let workcell = Workcell::from_str(&workcell_contents).unwrap();
    println!("{:?}", workcell);
}
