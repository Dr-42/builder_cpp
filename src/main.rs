use builder_cpp::utils;
fn main() {
    utils::log(utils::LogLevel::Info, "Hello, world!");
    let (build_config, targets) = utils::parse_config("./config_win32.toml");
    println!("{:?}", build_config);
    println!("{:?}", targets);
}
