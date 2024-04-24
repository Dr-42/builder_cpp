use std::{collections::{HashMap, HashSet}, fs::File, io::Read, path::Path};
use toml::{Table, Value};

use super::log::{log, LogLevel};

//Toml utils
/// Struct descibing the build config of the local project
#[derive(Debug)]
pub struct BuildConfig {
    pub compiler: String,
    pub packages: Vec<String>,
    pub pre_build: Option<String>,
    pub post_build: Option<String>,
}

/// Struct describing the target config of the local project
#[derive(Debug, Clone)]
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
            log(
                LogLevel::Error,
                &format!("Could not read src dir: {}", path),
            );
            std::process::exit(1);
        });
        for entry in src_entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                if file_name.ends_with(".cpp")
                    || file_name.ends_with(".c")
                    || file_name.ends_with(".cu")
                {
                    src_names.push(file_name.to_string());
                }
            } else if path.is_dir() {
                let dir_name = path.to_str().unwrap().replace('\\', "/");
                let mut dir_src_names = TargetConfig::get_src_names(&dir_name);
                src_names.append(&mut dir_src_names);
            }
        }
        src_names
    }

    fn arrange_targets(targets: Vec<TargetConfig>) -> Vec<TargetConfig> {
        let mut proj_indices: HashMap<&String, usize> = HashMap::new();
        for (i, proj) in targets.iter().enumerate() {
            proj_indices.insert(&proj.name, i);
        }

        let mut visited: HashSet<usize> = HashSet::new();

        let mut result: Vec<TargetConfig> = Vec::new();

        fn dfs(proj_index: usize, projects: &Vec<TargetConfig>, proj_indices: &HashMap<&String, usize>, visited: &mut HashSet<usize>, result: &mut Vec<TargetConfig>) {
            if visited.contains(&proj_index) {
                return;
            }
            visited.insert(proj_index);
            for dep_name in &projects[proj_index].deps {
                if let Some(&dep_index) = proj_indices.get(dep_name) {
                    dfs(dep_index, projects, proj_indices, visited, result);
                }
            }
            result.push(projects[proj_index].clone());
        }
        for (i, _) in targets.iter().enumerate() {
            dfs(i, &targets, &proj_indices, &mut visited, &mut result);
        }
        result
    }
}

/// This function is used to parse the config file of local project
/// # Arguments
/// * `path` - The path to the config file
/// * `check_dup_src` - If true, the function will check for duplicately named source files
pub fn parse_config(path: &str, check_dup_src: bool) -> (BuildConfig, Vec<TargetConfig>) {
    //open toml file and parse it into a string
    let mut file = File::open(path).unwrap_or_else(|_| {
        log(
            LogLevel::Error,
            &format!("Could not open config file: {}", path),
        );
        std::process::exit(1);
    });
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap_or_else(|_| {
        log(
            LogLevel::Error,
            &format!("Could not read config file: {}", path),
        );
        std::process::exit(1);
    });
    let config = contents.parse::<Table>().unwrap_or_else(|e| {
        log(
            LogLevel::Error,
            &format!("Could not parse config file: {}", path),
        );
        log(LogLevel::Error, &format!("Error: {}", e));
        std::process::exit(1);
    });

    let mut pkgs: Vec<String> = Vec::new();
    let empty_value = Value::Array(Vec::new());
    //pkgs is optional
    let pkgs_toml = config["build"]
        .as_table()
        .unwrap_or_else(|| {
            log(LogLevel::Error, "Could not find build in config file");
            std::process::exit(1);
        })
        .get("packages")
        .unwrap_or(&empty_value)
        .as_array()
        .unwrap_or_else(|| {
            log(LogLevel::Error, "packages is not an array");
            std::process::exit(1);
        });

    for pkg in pkgs_toml {
        pkgs.push(
            pkg.as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "packages are a vec of strings");
                    std::process::exit(1);
                })
                .to_string(),
        );
    }

    let pre_build = config["build"].get("pre_build").map(|x| {
        x.as_str()
            .unwrap_or_else(|| {
                log(
                    LogLevel::Error,
                    "pre_build is a string containing the command to run",
                );
                std::process::exit(1);
            })
            .to_string()
    });

    let post_build = config["build"].get("post_build").map(|x| {
        x.as_str()
            .unwrap_or_else(|| {
                log(
                    LogLevel::Error,
                    "post_build is a string containing the command to run",
                );
                std::process::exit(1);
            })
            .to_string()
    });

    //parse the string into a struct
    let build_config = BuildConfig {
        compiler: config["build"]["compiler"]
            .as_str()
            .unwrap_or_else(|| {
                log(LogLevel::Error, "Could not find compiler in config file");
                std::process::exit(1);
            })
            .to_string(),
        packages: pkgs,
        pre_build,
        post_build,
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
        let deps_toml = target
            .get("deps")
            .unwrap_or(&empty_value)
            .as_array()
            .unwrap_or_else(|| {
                log(LogLevel::Error, "Deps is not an array");
                std::process::exit(1);
            });
        for dep in deps_toml {
            deps.push(
                dep.as_str()
                    .unwrap_or_else(|| {
                        log(LogLevel::Error, "Deps are a vec of strings");
                        std::process::exit(1);
                    })
                    .to_string(),
            );
        }

        let target_config = TargetConfig {
            name: target["name"]
                .as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "Could not find name in config file");
                    std::process::exit(1);
                })
                .to_string(),
            src: target["src"]
                .as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "Could not find src in config file");
                    std::process::exit(1);
                })
                .to_string(),
            include_dir: target["include_dir"]
                .as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "Could not find include_dir in config file");
                    std::process::exit(1);
                })
                .to_string(),
            typ: target["type"]
                .as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "Could not find type in config file");
                    std::process::exit(1);
                })
                .to_string(),
            cflags: target["cflags"]
                .as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "Could not find cflags in config file");
                    std::process::exit(1);
                })
                .to_string(),
            libs: target["libs"]
                .as_str()
                .unwrap_or_else(|| {
                    log(LogLevel::Error, "Could not find libs in config file");
                    std::process::exit(1);
                })
                .to_string(),
            deps,
        };
        if target_config.typ != "exe" && target_config.typ != "dll" {
            log(LogLevel::Error, "Type must be exe or dll");
            std::process::exit(1);
        }
        tgt.push(target_config);
    }

    if tgt.is_empty() {
        log(LogLevel::Error, "No targets found");
        std::process::exit(1);
    }
    //Check for duplicate target names
    for i in 0..tgt.len() - 1 {
        for j in i + 1..tgt.len() {
            if tgt[i].name == tgt[j].name {
                log(
                    LogLevel::Error,
                    &format!("Duplicate target names found: {}", tgt[i].name),
                );
                std::process::exit(1);
            }
        }
    }

    if check_dup_src {
        for target in &tgt {
            let mut src_file_names = TargetConfig::get_src_names(&target.src);
            src_file_names.sort();
            if src_file_names.is_empty() {
                log(
                    LogLevel::Error,
                    &format!("No source files found for target: {}", target.name),
                );
                std::process::exit(1);
            }
            for i in 0..src_file_names.len() - 1 {
                if src_file_names[i] == src_file_names[i + 1] {
                    log(
                        LogLevel::Error,
                        &format!("Duplicate source files found for target: {}", target.name),
                    );
                    log(LogLevel::Error, "Source files must be unique");
                    log(
                        LogLevel::Error,
                        &format!("Duplicate file: {}", src_file_names[i]),
                    );
                    std::process::exit(1);
                }
            }
        }
    }

    let tgt_arranged = TargetConfig::arrange_targets(tgt);

    (build_config, tgt_arranged)
}
