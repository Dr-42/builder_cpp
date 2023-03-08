use builder_cpp::utils;
use builder_cpp::builder;
fn main() {
    utils::set_log_level(utils::LogLevel::Log);
    #[cfg(target_os = "linux")]
    let (build_config, targets) = utils::parse_config("./config_linux.toml");
    #[cfg(target_os = "windows")]
    let (build_config, targets) = utils::parse_config("./config_win32.toml");
    for target in targets {
        let target = builder::Target::new(&build_config, &target);
        target.build();
    }
}
