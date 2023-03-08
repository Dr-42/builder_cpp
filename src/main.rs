use builder_cpp::{utils, builder};
use std::env;
use std::path::Path;
use builder_cpp::builder::Target;

fn main() {
    #[cfg(target_os = "linux")]
    let (build_config, targets) = utils::parse_config("./config_linux.toml");
    #[cfg(target_os = "windows")]
    let (build_config, targets) = utils::parse_config("./config_win32.toml");

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
    if args.len() == 1 {
        print_help();
    }
    for arg in args {
        utils::set_log_level(utils::LogLevel::Log);
        if arg.starts_with("-ll=") {
            if arg == "-ll=Info" {
                utils::set_log_level(utils::LogLevel::Info);
            }
            if arg == "-ll=Log" {
                utils::set_log_level(utils::LogLevel::Log);
            }
            if arg == "-ll=Warn" {
                utils::set_log_level(utils::LogLevel::Warn);
            }
            if arg == "-ll=Error" {
                utils::set_log_level(utils::LogLevel::Error);
            }
        }

        if arg == "-c" {
            builder::clean(&build_config);
        }
        if arg == "-r" {
            if exe_target.is_none() {
                utils::log(utils::LogLevel::Error, "No executable target specified");
                std::process::exit(1);
            }
            let trgt = Target::new(&build_config, exe_target.unwrap());
            if !Path::new(&trgt.bin_path).exists() {
                builder::build(&build_config, &targets);
            }
            builder::build(&build_config, &targets);
            builder::run(&build_config, &exe_target.unwrap());
        }
        if arg == "-b" {
            builder::build(&build_config, &targets);
        }

        if arg == "-rb" {
            builder::clean(&build_config);
            builder::build(&build_config, &targets);
            builder::run(&build_config,&exe_target.unwrap());
        }
        if arg == "-h" {
            print_help();
        }
    }
}

fn print_help() {
    utils::log(utils::LogLevel::Log, "Usage: $ builder_cpp [options]");
    utils::log(utils::LogLevel::Log, "Options:");
    utils::log(utils::LogLevel::Log, "  -c\t\tClean the build directory");
    utils::log(utils::LogLevel::Log, "  -r\t\tRun the executable");
    utils::log(utils::LogLevel::Log, "  -b\t\tBuild the project");
    utils::log(utils::LogLevel::Log, "  -rb\t\tClean, build and run the project");
    utils::log(utils::LogLevel::Log, "  -ll=Log\tSet the log level to Log");
    utils::log(utils::LogLevel::Log, "  -ll=Warn\tSet the log level to Warn");
    utils::log(utils::LogLevel::Log, "  -ll=Error\tSet the log level to Error");
    utils::log(utils::LogLevel::Log, "  -h\t\tShow this help message");
}
