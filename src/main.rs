use builder_cpp::utils;
use builder_cpp::builder;
fn main() {
    let (build_config, targets) = utils::parse_config("./config_win32.toml");
    println!("{:?}", build_config);
    println!("{:?}", targets);
    for target in targets {
        let _target = builder::Target::new(&build_config, &target);
    }
}
