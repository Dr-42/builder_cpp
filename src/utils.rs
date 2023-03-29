//! This file contains various logging and toml parsing functions
//! used by the builder_cpp library
use std::{fs::File, io::Read, path::Path, process::Command};
use toml::{Table, Value};
use colored::Colorize;

#[cfg(target_os = "windows")]
static OBJ_DIR: &str  = ".bld_cpp/obj_win32";
#[cfg(target_os = "linux")]
static OBJ_DIR: &str  = ".bld_cpp/obj_linux";


//Log utils
#[derive(PartialEq, PartialOrd, Debug)]
/// This enum is used to represent the different log levels
pub enum LogLevel {
    Debug,
    Info,
    Log,
    Warn,
    Error,
}

/// This function is used to log messages to the console
/// # Arguments
/// * `level` - The log level of the message
/// * `message` - The message to log
/// # Example
/// ```
/// log(LogLevel::Info, "Hello World!");
/// log(LogLevel::Error, &format!("Something went wrong! {}", error));
/// ```
///
/// # Level setting
/// The log level can be set by setting the environment variable `BUILDER_CPP_LOG_LEVEL`
/// to one of the following values:
/// * `Debug`
/// * `Info`
/// * `Log`
/// * `Warn`
/// * `Error`
/// If the environment variable is not set, the default log level is `Log`
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
    if log_level == LogLevel::Debug {
        if level == LogLevel::Debug || level == LogLevel::Warn || level == LogLevel::Error {
            println!("{} {}", level_str, message);
        }
    } else if level >= log_level {
        println!("{} {}", level_str, message);
    }
}

//Toml utils
/// Struct descibing the build config of the local project
#[derive(Debug)]
pub struct BuildConfig {
    pub compiler: String,
    pub packages: Vec<String>,
}

/// Struct describing the target config of the local project
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

impl TargetConfig {
    /// Returns a vec of all filenames ending in .cpp or .c in the src directory
    /// # Arguments
    /// * `path` - The path to the src directory
    fn get_src_names(path: &str) -> Vec<String> {
        let mut src_names = Vec::new();
        let src_path = Path::new(&path);
        let src_entries = std::fs::read_dir(src_path).unwrap_or_else(|_| {
            log(LogLevel::Error, &format!("Could not read src dir: {}", path));
            std::process::exit(1);
        });
        for entry in src_entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                if file_name.ends_with(".cpp") || file_name.ends_with(".c") {
                    src_names.push(file_name.to_string());
                }
            } else if path.is_dir() {
                let dir_name = path.to_str().unwrap().replace("\\", "/");
                let mut dir_src_names = TargetConfig::get_src_names(&dir_name);
                src_names.append(&mut dir_src_names);
            }
        }
        src_names
    }
}

/// This function is used to parse the config file of local project
/// # Arguments
/// * `path` - The path to the config file
/// * `check_dup_src` - If true, the function will check for duplicately named source files
pub fn parse_config(path: &str, check_dup_src: bool) -> (BuildConfig, Vec<TargetConfig>) {
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


    let mut pkgs: Vec<String> = Vec::new();
    let empty_value = Value::Array(Vec::new());
    //pkgs is optional
    let pkgs_toml = config["build"].as_table().unwrap_or_else(|| {
        log(LogLevel::Error, "Could not find build in config file");
        std::process::exit(1);})
        .get("packages").unwrap_or_else(|| &empty_value)
        .as_array()
        .unwrap_or_else(|| {
            log(LogLevel::Error, "packages is not an array");
            std::process::exit(1);
        });

    for pkg in pkgs_toml            {
        pkgs.push(pkg.as_str().unwrap_or_else(|| {
            log(LogLevel::Error, "packages are a vec of strings");
            std::process::exit(1);
        }).to_string());
    }

    //parse the string into a struct
    let build_config = BuildConfig {
        compiler: config["build"]["compiler"].as_str().unwrap_or_else(|| {
            log(LogLevel::Error, "Could not find compiler in config file");
            std::process::exit(1);
        }).to_string(),
        packages: pkgs,
    };

    let mut tgt = Vec::new();
    let targets = config["targets"].as_array().unwrap_or_else(|| {
        log(LogLevel::Error, "Could not find targets in config file");
        std::process::exit(1);
    });

    for target in targets {
        let mut deps: Vec<String> = Vec::new();
        let empty_value = Value::Array(Vec::new());
        //deps is optional
        let deps_toml = target.get("deps").unwrap_or_else(|| { &empty_value })
            .as_array()
            .unwrap_or_else(|| {
                log(LogLevel::Error, "Deps is not an array");
                std::process::exit(1);
            })
        ;
        for dep in deps_toml            {
            deps.push(dep.as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Deps are a vec of strings");
                std::process::exit(1);
            }).to_string());
        }

        let target_config = TargetConfig {
            name: target["name"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find name in config file");
                std::process::exit(1);
            }).to_string(),
            src: target["src"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find src in config file");
                std::process::exit(1);
            }).to_string(),
            include_dir: target["include_dir"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find include_dir in config file");
                std::process::exit(1);
            }).to_string(),
            typ: target["type"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find type in config file");
                std::process::exit(1);
            }).to_string(),
            cflags: target["cflags"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find cflags in config file");
                std::process::exit(1);
            }).to_string(),
            libs: target["libs"].as_str().unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find libs in config file");
                std::process::exit(1);
            }).to_string(),
            deps,
        };
        if target_config.typ != "exe" && target_config.typ != "dll" {
            log(LogLevel::Error, "Type must be exe or dll");
            std::process::exit(1);
        }
        tgt.push(target_config);
    }

    if tgt.len() == 0 {
        log(LogLevel::Error, "No targets found");
        std::process::exit(1);
    }
    let original_len = tgt.len();
    tgt.sort_by_key(|t| t.name.clone());
    tgt.dedup_by_key(|t| t.name.clone());
    let dedup_len = tgt.len();
    if original_len != dedup_len {
        log(LogLevel::Error, "Duplicate targets found");
        log(LogLevel::Error, "Target names must be unique");
        std::process::exit(1);
    }
    if check_dup_src {
        for target in &tgt {
            let mut src_file_names = TargetConfig::get_src_names(&target.src);
            src_file_names.sort();
            if src_file_names.len() == 0 {
                log(LogLevel::Error, &format!("No source files found for target: {}", target.name));
                std::process::exit(1);
            }
            for i in 0..src_file_names.len() - 1 {
                if src_file_names[i] == src_file_names[i + 1] {
                    log(LogLevel::Error, &format!("Duplicate source files found for target: {}", target.name));
                    log(LogLevel::Error, &format!("Source files must be unique"));
                    log(LogLevel::Error, &format!("Duplicate file: {}", src_file_names[i]));
                    std::process::exit(1);
                }
            }
        }
    }

    (build_config, tgt)
}

#[derive(Debug)]
/// Represents a package
pub struct Package {
    pub name: String,
    pub repo: String,
    pub branch: String,
    pub build_config: BuildConfig,
    pub target_configs: Vec<TargetConfig>,
}

impl Package {
    /// Creates a new package
    pub fn new(name: String, repo: String, branch: String, build_config: BuildConfig, target_configs: Vec<TargetConfig>) -> Package {
        Package {
            name,
            repo,
            branch,
            build_config,
            target_configs,
        }
    }

    /// Updates the package to latest commit
    pub fn update(&self) {
        let mut cmd = String::from("cd");
        cmd.push_str(&format!(" ./.bld_cpp/sources/{}", self.name));
        log(LogLevel::Log, &format!("Updating package: {}", self.name));
        cmd.push_str(" &&");
        cmd.push_str(" git");
        cmd.push_str(" pull");
        cmd.push_str(" origin");
        cmd.push_str(&format!(" {}", self.branch));
        let com = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .unwrap_or_else(|e| {
                log(LogLevel::Error, &format!("Failed to update package: {}", e));
                std::process::exit(1);
            });
        if com.status.success() {
            log(LogLevel::Log, &format!("Successfully updated package: {}", self.name));
            log(LogLevel::Log, &format!("Output: {}", String::from_utf8_lossy(&com.stdout)).replace("\r", "").replace("\n", ""));
        } else {
            log(LogLevel::Error, &format!("Failed to update package: {}", String::from_utf8_lossy(&com.stderr)));
            std::process::exit(1);
        }
    }

    /// Restores package to last offline commit
    pub fn restore(&self) {
        let mut cmd = String::from("cd");
        cmd.push_str(&format!(" ./.bld_cpp/sources/{}", self.name));
        log(LogLevel::Log, &format!("Updating package: {}", self.name));
        cmd.push_str(" &&");
        cmd.push_str(" git");
        cmd.push_str(" reset");
        cmd.push_str(" --hard");
        cmd.push_str(&format!(" {}", self.branch));
        let com = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .unwrap_or_else(|e| {
                log(LogLevel::Error, &format!("Failed to restore package: {}", e));
                std::process::exit(1);
            });
        if com.status.success() {
            log(LogLevel::Log, &format!("Successfully restored package: {}", self.name));
            log(LogLevel::Log, &format!("Output: {}", String::from_utf8_lossy(&com.stdout)).replace("\r", "").replace("\n", ""));
        } else {
            log(LogLevel::Error, &format!("Failed to restore package: {}", String::from_utf8_lossy(&com.stderr)));
            std::process::exit(1);
        }
    }

    /// Parses a package contained in a folder
    /// The folder must contain a config timl file
    /// # Arguments
    /// * `path` - The path to the folder containing the package
    pub fn parse_packages(path: &str) -> Vec<Package> {
        let mut packages: Vec<Package> = Vec::new();
        //initialize fields
        let mut name = String::new();
        let mut repo = String::new();
        let mut branch = String::new();
        let mut build_config = BuildConfig {
            compiler: String::new(),
            packages: Vec::new(),
        };
        let mut target_configs = Vec::new();

        //parse the root toml file
        let (build_config_toml, _) = parse_config(path, false);
        for package in build_config_toml.packages {
            let deets = package.split_whitespace().collect::<Vec<&str>>();
            if deets.len() != 2 {
                log(LogLevel::Error, "Packages must be in the form of \"<git_repo> <branch>\"");
                std::process::exit(1);
            }
            repo = deets[0].to_string().replace(",", "");
            branch = deets[1].to_string();

            name = repo.split("/").collect::<Vec<&str>>()[1].to_string();
            let source_dir = format!("./.bld_cpp/sources/{}/", name);
            if !Path::new(&source_dir).exists() {
                Command::new("mkdir")
                    .arg("-p")
                    .arg(&source_dir)
                    .output()
                    .expect("Failed to execute mkdir");
                if !Path::new(&source_dir).exists() {
                    log(LogLevel::Error, &format!("Failed to create {}", source_dir));
                    std::process::exit(1);
                } else {
                    log(LogLevel::Info, &format!("Created {}", source_dir));
                }
                log(LogLevel::Log, &format!("Cloning {} into {}", repo, source_dir));
                let repo_https = format!("https://github.com/{}", repo);
                let mut cmd = Command::new("git");
                cmd.arg("clone")
                    .arg("--branch")
                    .arg(&branch)
                    .arg(&repo_https)
                    .arg(&source_dir);
                let output = cmd.output().expect("Failed to execute git clone");
                if !output.status.success() {
                    log(LogLevel::Error, &format!("Failed to clone {} branch {} into {}", repo, branch, source_dir));
                    std::process::exit(1);
                }
            }
            #[cfg(target_os = "linux")]
            let pkg_toml = format!("{}/config_linux.toml", source_dir).replace("//", "/");
            #[cfg(target_os = "windows")]
            let pkg_toml = format!("{}/config_win32.toml", source_dir).replace("//", "/");

            let (pkg_bld_config_toml, pkg_targets_toml) = parse_config(&pkg_toml, false);
            log(LogLevel::Info, &format!("Parsed {}", pkg_toml));

            if pkg_bld_config_toml.packages.len() > 0 {
                for foreign_package in Package::parse_packages(&pkg_toml){
                    packages.push(foreign_package);
                }
            }

            build_config = pkg_bld_config_toml;
            build_config.compiler = build_config_toml.compiler.clone();
            if !Path::new(OBJ_DIR).exists() {
                let cmd = Command::new("mkdir")
                    .arg("-p")
                    .arg(OBJ_DIR)
                    .output();
                if cmd.is_err() {
                    log(LogLevel::Error, &format!("Failed to create {}", OBJ_DIR));
                    std::process::exit(1);
                }
                log(LogLevel::Info, &format!("Created {}", OBJ_DIR));
            }

            let tgt_configs = pkg_targets_toml;
            for mut tgt in tgt_configs {
                if tgt.typ != "dll" {
                    continue;
                }
                tgt.src = format!("{}/{}", source_dir, tgt.src).replace("\\", "/").replace("/./", "/").replace("//", "/");
                let old_inc_dir = tgt.include_dir.clone();
                tgt.include_dir = format!("./.bld_cpp/includes/{}", name).replace("\\", "/").replace("/./", "/").replace("//", "/");
                if !Path::new(&tgt.include_dir).exists() {
                    let cmd = Command::new("mkdir")
                        .arg("-p")
                        .arg(&tgt.include_dir)
                        .output();
                    if cmd.is_err() {
                        log(LogLevel::Error, &format!("Failed to create {}", tgt.include_dir));
                        std::process::exit(1);
                    }
                    log(LogLevel::Info, &format!("Created {}", tgt.include_dir));
                    let mut cm = String::new();
                    cm.push_str("cp -r ");
                    cm.push_str(&format!("{}/{}/* ", source_dir, old_inc_dir).replace("\\", "/").replace("/./", "/").replace("//", "/"));
                    cm.push_str(&tgt.include_dir);
                    cm.push_str("/ ");
                    let cmd = Command::new("sh")
                        .arg("-c")
                        .arg(&cm)
                        .output();
                    if cmd.is_err() {
                        log(LogLevel::Error, &format!("Failed to create {}", tgt.include_dir));
                        std::process::exit(1);
                    }
                }
                target_configs.push(tgt);
            }
        }

        packages.push(Package::new(name, repo, branch, build_config, target_configs));
        packages.sort_by_key(|a| a.name.clone());
        packages.dedup_by_key(|a| a.name.clone());
        packages
    }
}
