use builder_cpp::{utils, builder};
use std::env;
use std::path::Path;

static VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--version".to_string()) {
        utils::log(utils::LogLevel::Log, &format!("builder_cpp v{}", VERSION));
        std::process::exit(0);
    }

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    if args.contains(&"--init".to_string()) {
        utils::log(utils::LogLevel::Log, "Initializing project...");
        builder::init();
        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    let (build_config, targets) = utils::parse_config("./config_linux.toml", true);
    #[cfg(target_os = "windows")]
    let (build_config, targets) = utils::parse_config("./config_win32.toml", true);

    #[cfg(target_os = "linux")]
    let packages = utils::Package::parse_packages("./config_linux.toml");
    #[cfg(target_os = "windows")]
    let packages = utils::Package::parse_packages("./config_win32.toml");

    let mut num_exe = 0;
    let mut exe_target : Option<&utils::TargetConfig> = None;
    if targets.len() == 0 {
        utils::log(utils::LogLevel::Error, "No targets in config");
        std::process::exit(1);
    } else {
        //Allow only one exe and set it as the exe_target
        for target in &targets {
            if target.typ == "exe" {
                num_exe += 1;
                exe_target = Some(target);
            }
        }
    }

    if num_exe != 1 || exe_target.is_none() {
        utils::log(utils::LogLevel::Error, "Exactly one executable target must be specified");
        std::process::exit(1);
    }

    let args: Vec<String> = env::args().collect();
    let mut gen_cc = false;
    let mut valid_arg = false;
    if args.contains(&"--gen-cc".to_string()) {
        gen_cc = true;
        use std::fs;
        if !Path::new("./compile_commands.json").exists() {
            fs::File::create(Path::new("./compile_commands.json")).unwrap();
        } else {
            fs::remove_file(Path::new("./compile_commands.json")).unwrap();
            fs::File::create(Path::new("./compile_commands.json")).unwrap();
        }
        valid_arg = true;
    }

    if args.contains(&"--clean-packages".to_string()) {
        utils::log(utils::LogLevel::Log, "Cleaning packages...");
        builder::clean_packages(&packages);
        valid_arg = true;
    }

    if args.contains(&"--update-packages".to_string()) {
        utils::log(utils::LogLevel::Log, "Updating packages...");
        for package in &packages {
            package.update();
        }
        valid_arg = true;
    }

    if args.contains(&"--restore-packages".to_string()) {
        utils::log(utils::LogLevel::Log, "Restoring packages...");
        for package in &packages {
            package.restore();
        }
        valid_arg = true;
    }

    for (i, arg) in args.iter().enumerate() {
        if arg.starts_with("-") {
            if i == 0 {
                continue;
            }

            if arg.starts_with("--") {
                continue;
            }
            if arg.len() < 2 {
                utils::log(utils::LogLevel::Error, &format!("Invalid argument: {}", arg));
                std::process::exit(1);
            }

            let arg = arg.as_str()[1..].to_string();

            let all_letters = "crb";
            for letter in arg.chars() {
                if !all_letters.contains(letter) {
                    utils::log(utils::LogLevel::Error, &format!("Invalid argument: {}", arg));
                    std::process::exit(1);
                }
            }
            if arg.contains('c') {
                utils::log(utils::LogLevel::Log, "Cleaning...");
                builder::clean(&targets);
                valid_arg = true;
            }
            if arg.contains('b') {
                utils::log(utils::LogLevel::Log, "Building project...");
                builder::build(&build_config, &targets, gen_cc, &packages);
                valid_arg = true;
            }
            if arg.contains('r') {
                valid_arg = true;
                if exe_target.is_none() {
                    utils::log(utils::LogLevel::Error, "No executable target specified");
                    std::process::exit(1);
                }
                utils::log(utils::LogLevel::Log, "Running executable...");
                builder::run(&build_config, &exe_target.unwrap(), &targets, &packages);
            }

        }
    }
    if !valid_arg {
        utils::log(utils::LogLevel::Log, "No valid arguments specified ");
        print_help();
        std::process::exit(1);
    }
}

fn print_help() {
    utils::log(utils::LogLevel::Log, "Usage: $ builder_cpp [options]");
    utils::log(utils::LogLevel::Log, "Options:");
    utils::log(utils::LogLevel::Log, "\t-c\t\tClean the build directory");
    utils::log(utils::LogLevel::Log, "\t-r\t\tRun the executable");
    utils::log(utils::LogLevel::Log, "\t-b\t\tBuild the project");
    utils::log(utils::LogLevel::Log, "\t-h\t\tShow this help message");
    utils::log(utils::LogLevel::Log, "");

    utils::log(utils::LogLevel::Log, "\t--help\t\t\tShow this help message");
    utils::log(utils::LogLevel::Log, "\t--gen-cc\t\tGenerate compile_commands.json");
    utils::log(utils::LogLevel::Log, "\t--clean-packages\tClean the package binaries");
    utils::log(utils::LogLevel::Log, "\t--update-packages\tUpdate the packages");
    utils::log(utils::LogLevel::Log, "\t--restore-packages\tRestore the packages");
    utils::log(utils::LogLevel::Log, "\t--version\t\tShow the version");
    utils::log(utils::LogLevel::Log, "Environment variables:");
    utils::log(utils::LogLevel::Log, "\tBUILDER_CPP_LOG_LEVEL");
    utils::log(utils::LogLevel::Log, "\t\tValid values are: Debug, Log, Info, Warn, Error");
}
