use std::{fs::File, io::Read};
use toml::{Table, Value};
use colored::Colorize;

//Log utils
#[derive(PartialEq, PartialOrd, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Log,
    Warn,
    Error,
}

pub fn log(level: LogLevel, message: &str) {
    let level_str = match level {
        LogLevel::Debug => "[DEBUG]".purple(),
        LogLevel::Info => "[INFO]".blue(),
        LogLevel::Log => "[LOG]".green(),
        LogLevel::Warn => "[WARN]".yellow(),
        LogLevel::Error => "[ERROR]".red(),
    };
    let log_level = match std::env::var("BUILDER_CPP_LOG_LEVEL") {
        Ok(val) => {
            if val == "Debug" {
                LogLevel::Debug
            } else if val == "Info" {
                LogLevel::Info
            } else if val == "Log" {
                LogLevel::Log
            } else if val == "Warn" {
                LogLevel::Warn
            } else if val == "Error" {
                LogLevel::Error
            } else {
                LogLevel::Log
            }
        }
        Err(_) => LogLevel::Log,
    };
    if level >= log_level {
        println!("{} {}", level_str, message);
    }
}

//Toml utils
#[derive(Debug)]
pub struct BuildConfig {
    pub compiler: String,
    pub build_dir: String,
    pub obj_dir: String,
}

#[derive(Debug)]
pub struct TargetConfig {
    pub name: String,
    pub src: String,
    pub include_dir: String,
    pub typ: String,
    pub cflags: String,
    pub libs: String,
    pub deps: Vec<String>,
}

pub fn parse_config(path: &str) -> (BuildConfig, Vec<TargetConfig>) {
    //open toml file and parse it into a string
    let mut file = File::open(path).unwrap_or_else(|_| {
        log(LogLevel::Error, &format!("Could not open config file: {}", path));
        std::process::exit(1);
    });
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap_or_else(|_| {
        log(LogLevel::Error, &format!("Could not read config file: {}", path));
        std::process::exit(1);
    });
    let config = contents.parse::<Table>().unwrap_or_else(|e| {
        log(LogLevel::Error, &format!("Could not parse config file: {}", path));
        log(LogLevel::Error, &format!("Error: {}", e));
        std::process::exit(1);
    });
    
    //parse the string into a struct
    let build_config = BuildConfig {
        compiler: config["build"]["compiler"].as_str().unwrap_or_else(|| {
            log(LogLevel::Error, "Could not find compiler in config file");
            std::process::exit(1);
        }).to_string(),
        build_dir: config["build"]["build_dir"].as_str().unwrap_or_else(|| {
            log(LogLevel::Error, "Could not find build_dir in config file");
            std::process::exit(1);
        }).to_string(),
        obj_dir: config["build"]["obj_dir"].as_str().unwrap_or_else(|| {
            log(LogLevel::Error, "Could not find obj_dir in config file");
            std::process::exit(1);
        }).to_string(),
    };

    let mut targets = Vec::new();

    for target in config["targets"].as_array().unwrap_or_else(|| {
        log(LogLevel::Error, "Could not find targets in config file");
        std::process::exit(1);
    })
    {
        let mut deps: Vec<String> = Vec::new();

        let empty_value = Value::Array(Vec::new());
        //deps is optional
        let deps_toml = target.get("deps").unwrap_or_else(|| {
            &empty_value
        });
        for dep in deps_toml.as_array().unwrap_or_else(|| {
            log(LogLevel::Error, "Deps is not an array");
            std::process::exit(1);
        })
        {
            deps.push(dep.as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Deps are a vec of strings");
                std::process::exit(1);
            }).to_string());
        }
            
        let target_config = TargetConfig {
            name: target["name"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find name in confiig file");
                std::process::exit(1);
            }).to_string(),
            src: target["src"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find src in confiig file");
                std::process::exit(1);
            }).to_string(),
            include_dir: target["include_dir"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find include_dir in confiig file");
                std::process::exit(1);
            }).to_string(),
            typ: target["type"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find type in confiig file");
                std::process::exit(1);
            }).to_string(),
            cflags: target["cflags"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find cflags in confiig file");
                std::process::exit(1);
            }).to_string(),
            libs: target["libs"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find libs in confiig file");
                std::process::exit(1);
            }).to_string(),
            deps,
        };
        if target_config.typ != "exe" && target_config.typ != "dll" {
            log(LogLevel::Error, "Type must be exe or dll");
            std::process::exit(1);
        }
        targets.push(target_config);
    }

    (build_config, targets)
}
