use crate::builder::Target;
use crate::global_config::GlobalConfig;
use crate::utils::{self, log, BuildConfig, LogLevel, Package, TargetConfig};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

static BUILD_DIR: &str = ".bld_cpp/bin";
#[cfg(target_os = "windows")]
static OBJ_DIR: &str = ".bld_cpp/obj_win32";
#[cfg(target_os = "linux")]
static OBJ_DIR: &str = ".bld_cpp/obj_linux";
#[cfg(target_os = "android")]
static OBJ_DIR: &str = ".bld_cpp/obj_linux";

///Cleans the local targets
/// # Arguments
/// * `targets` - A vector of targets to clean
pub fn clean(targets: &Vec<TargetConfig>) {
    if Path::new(".bld_cpp").exists() {
        fs::create_dir_all(".bld_cpp").unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not remove binary directory: {}", why),
            );
        });
    }
    if Path::new(OBJ_DIR).exists() {
        fs::remove_dir_all(OBJ_DIR).unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not remove object directory: {}", why),
            );
        });
        log(LogLevel::Info, &format!("Cleaning: {}", OBJ_DIR));
    }
    for target in targets {
        //remove hashes
        #[cfg(target_os = "windows")]
        let hash_path = format!(".bld_cpp/{}.win32.hash", &target.name);
        #[cfg(target_os = "linux")]
        let hash_path = format!(".bld_cpp/{}.linux.hash", &target.name);
        #[cfg(target_os = "android")]
        let hash_path = format!(".bld_cpp/{}.linux.hash", &target.name);

        if Path::new(&hash_path).exists() {
            fs::remove_file(&hash_path).unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not remove hash file: {}", why),
                );
            });
            log(LogLevel::Info, &format!("Cleaning: {}", &hash_path));
        }
        if Path::new(BUILD_DIR).exists() {
            let mut bin_name = String::new();
            bin_name.push_str(BUILD_DIR);
            bin_name.push('/');
            bin_name.push_str(&target.name);
            #[cfg(target_os = "windows")]
            if target.typ == "exe" {
                bin_name.push_str(".exe");
            } else if target.typ == "dll" {
                bin_name.push_str(".dll");
            }
            #[cfg(target_os = "linux")]
            if target.typ == "exe" {
                bin_name.push_str("");
            } else if target.typ == "dll" {
                bin_name.push_str(".so");
            }
            #[cfg(target_os = "android")]
            if target.typ == "exe" {
                bin_name.push_str("");
            } else if target.typ == "dll" {
                bin_name.push_str(".so");
            }
            if Path::new(&bin_name).exists() {
                fs::remove_file(&bin_name).unwrap_or_else(|why| {
                    log(
                        LogLevel::Error,
                        &format!("Could not remove binary file: {}", why),
                    );
                });
                log(LogLevel::Log, &format!("Cleaning: {}", &bin_name));
            } else {
                log(
                    LogLevel::Log,
                    &format!("Binary file does not exist: {}", &bin_name),
                );
            }
        }
    }
}

///Cleans the downloaded packages
/// # Arguments
/// * `packages` - A vector of packages to clean
pub fn clean_packages(packages: &Vec<Package>) {
    for pack in packages {
        for target in &pack.target_configs {
            #[cfg(target_os = "windows")]
            let pack_bin_path = format!("{}/{}.dll", BUILD_DIR, &target.name);
            #[cfg(target_os = "linux")]
            let pack_bin_path = format!("{}/{}.so", BUILD_DIR, &target.name);
            #[cfg(target_os = "android")]
            let pack_bin_path = format!("{}/{}.so", BUILD_DIR, &target.name);

            if !Path::new(&pack_bin_path).exists() {
                log(
                    LogLevel::Log,
                    &format!("Package binary does not exist: {}", &pack_bin_path),
                );
                continue;
            }
            let cmd_str = format!("rm {}", &pack_bin_path);
            log(LogLevel::Debug, cmd_str.as_str());
            let output = Command::new("sh")
                .arg("-c")
                .arg(&cmd_str)
                .output()
                .expect("failed to execute process");
            if output.status.success() {
                log(
                    LogLevel::Log,
                    &format!("Cleaned package: {} of {}", &pack.name, &pack.repo),
                );
            } else {
                log(
                    LogLevel::Error,
                    &format!("Could not clean package: {} of {}", &pack.name, &pack.repo),
                );
            }
        }
    }
}

///Builds all targets
/// # Arguments
/// * `build_config` - The local build configuration
/// * `targets` - A vector of targets to build
/// * `gen_cc` - Whether to generate a compile_commands.json file
pub fn build(
    build_config: &BuildConfig,
    targets: &Vec<TargetConfig>,
    gen_cc: bool,
    gen_vsc: bool,
    packages: &Vec<Package>,
) {
    if !Path::new("./.bld_cpp").exists() {
        fs::create_dir(".bld_cpp").unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not create bld_cpp directory: {}", why),
            );
            std::process::exit(1);
        });
    }
    if gen_cc {
        let mut cc_file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open("compile_commands.json")
            .unwrap_or_else(|why| {
                log(LogLevel::Error, &format!("Could not open cc file: {}", why));
                std::process::exit(1);
            });
        cc_file.write_all(b"[").unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not write to cc file: {}", why),
            );
            std::process::exit(1);
        });
    }

    if gen_vsc {
        let mut vsc_file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(".vscode/c_cpp_properties.json")
            .unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not open vsc file: {}", why),
                );
                std::process::exit(1);
            });

        let mut inc_dirs: Vec<String> = targets.iter().map(|t| t.include_dir.clone()).collect();
        for package in packages {
            for target in &package.target_configs {
                inc_dirs.push(target.include_dir.clone());
            }
        }
        let compiler_path: String = build_config.compiler.clone();
        let mut intellimode: String = String::new();
        if compiler_path == "gcc" || compiler_path == "g++" {
            intellimode = "gcc-x64".to_string();
        } else if compiler_path == "clang" || compiler_path == "clang++" {
            intellimode = "clang-x64".to_string();
        } else {
            log(
                LogLevel::Error,
                &format!("Unsupported compiler: {}", compiler_path),
            );
        }

        #[cfg(target_os = "windows")]
        let compiler_path = Command::new("sh")
            .arg("-c")
            .arg(&format!("where {}", &compiler_path))
            .output()
            .expect("failed to execute process")
            .stdout;

        #[cfg(target_os = "windows")]
        //Pick the first compiler path
        let compiler_path = String::from_utf8(compiler_path)
            .unwrap()
            .split('\n')
            .collect::<Vec<&str>>()[0]
            .to_string()
            .replace('\r', "")
            .replace('\\', "/");

        #[cfg(target_os = "windows")]
        let vsc_json = format!(
            r#"{{
    "configurations": [
        {{
            "name": "Win32",
            "includePath": [
                "{}"
            ],
            "defines": [
                "_DEBUG",
                "UNICODE",
                "_UNICODE"
            ],
            "compilerPath": "{}",
            "cStandard": "c11",
            "cppStandard": "c++17",
            "intelliSenseMode": "windows-{}"
        }}
    ],
    "version": 4
}}"#,
            inc_dirs.join("\",\n\t\t\t\t\""),
            compiler_path,
            intellimode
        );

        #[cfg(target_os = "linux")]
        let compiler_path = Command::new("sh")
            .arg("-c")
            .arg(&format!("which {}", &compiler_path))
            .output()
            .expect("failed to execute process")
            .stdout;

        #[cfg(target_os = "linux")]
        let compiler_path = String::from_utf8(compiler_path).unwrap().replace('\n', "");

        #[cfg(target_os = "linux")]
        let vsc_json = format!(
            r#"{{
    "configurations": [
        {{
            "name": "Linux",
            "includePath": [
                "{}"
            ],
            "defines": [
                "_DEBUG",
                "UNICODE",
                "_UNICODE"
            ],
            "compilerPath": "{}",
            "cStandard": "c11",
            "cppStandard": "c++17",
            "intelliSenseMode": "linux-{}"
        }}
    ],
    "version": 4
}}"#,
            inc_dirs.join("\",\n\t\t\t\t\""),
            compiler_path,
            intellimode
        );
        #[cfg(target_os = "android")]
        let compiler_path = Command::new("sh")
            .arg("-c")
            .arg(&format!("which {}", &compiler_path))
            .output()
            .expect("failed to execute process")
            .stdout;

        #[cfg(target_os = "android")]
        let compiler_path = String::from_utf8(compiler_path).unwrap().replace('\n', "");

        #[cfg(target_os = "android")]
        let vsc_json = format!(
            r#"{{
    "configurations": [
        {{
            "name": "Linux",
            "includePath": [
                "{}"
            ],
            "defines": [
                "_DEBUG",
                "UNICODE",
                "_UNICODE"
            ],
            "compilerPath": "{}",
            "cStandard": "c11",
            "cppStandard": "c++17",
            "intelliSenseMode": "linux-{}"
        }}
    ],
    "version": 4
}}"#,
            inc_dirs.join("\",\n\t\t\t\t\""),
            compiler_path,
            intellimode
        );

        //Write to file
        vsc_file
            .write_all(vsc_json.as_bytes())
            .unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not write to vsc file: {}", why),
                );
                std::process::exit(1);
            });
    }

    for target in targets {
        let mut tgt = Target::new(build_config, target, targets, packages);
        tgt.build(gen_cc);
    }
    if gen_cc {
        let mut cc_file = fs::OpenOptions::new()
            .write(true)
            .read(true)
            .append(true)
            .open("compile_commands.json")
            .unwrap_or_else(|why| {
                log(LogLevel::Error, &format!("Could not open cc file: {}", why));
                std::process::exit(1);
            });
        cc_file.write_all(b"]").unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not write to cc file: {}", why),
            );
            std::process::exit(1);
        });
    }
    log(LogLevel::Info, "Build complete");
}

///Runs the exe target
/// # Arguments
/// * `build_config` - The local build configuration
/// * `exe_target` - The exe target to run
/// * `targets` - A vector of targets
/// * `packages` - A vector of packages
pub fn run(
    bin_args: Option<Vec<&str>>,
    build_config: &BuildConfig,
    exe_target: &TargetConfig,
    targets: &Vec<TargetConfig>,
    packages: &Vec<Package>,
) {
    let trgt = Target::new(build_config, exe_target, targets, packages);
    if !Path::new(&trgt.bin_path).exists() {
        log(
            LogLevel::Error,
            &format!("Could not find binary: {}", &trgt.bin_path),
        );
        std::process::exit(1);
    }
    log(LogLevel::Log, &format!("Running: {}", &trgt.bin_path));
    let mut cmd = std::process::Command::new(&trgt.bin_path);
    if let Some(bin_args) = bin_args {
        for arg in bin_args {
            cmd.arg(arg);
        }
    }
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let output = cmd.output();
    if output.is_ok() {
        log(LogLevel::Info, &format!("  Success: {}", &trgt.bin_path));
    } else {
        log(LogLevel::Error, &format!("  Error: {}", &trgt.bin_path));
        std::process::exit(1);
    }
}

///Initialises a new project in the current directory
pub fn init(project_name: &str, is_c: Option<bool>, config: GlobalConfig) {
    if Path::new(project_name).exists() {
        log(LogLevel::Error, &format!("{} already exists", project_name));
        log(LogLevel::Error, "Cannot initialise project");
        std::process::exit(1);
    }
    //initialise git repo in project directory
    let mut cmd = std::process::Command::new("git");
    cmd.arg("init").arg(project_name);
    let output = cmd.output();
    if output.is_err() {
        log(LogLevel::Error, "Could not initialise git repo");
        log(LogLevel::Error, &format!("{}", output.err().unwrap()));
        std::process::exit(1);
    }

    #[cfg(target_os = "windows")]
    let config_file = project_name.to_owned() + "/config_win32.toml";
    #[cfg(target_os = "linux")]
    let config_file = project_name.to_owned() + "/config_linux.toml";
    #[cfg(target_os = "android")]
    let config_file = project_name.to_owned() + "/config_linux.toml";

    if Path::new(&config_file).exists() {
        log(LogLevel::Error, &format!("{} already exists", config_file));
        log(LogLevel::Error, "Cannot initialise project");
        std::process::exit(1);
    }

    let mut config_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(config_file)
        .unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not create config file: {}", why),
            );
            std::process::exit(1);
        });

    let c_compiler = match config.get_default_compiler().as_str() {
        "gcc" => "gcc",
        "clang" => "clang",
        _ => {
            log(LogLevel::Error, "Invalid default compiler");
            std::process::exit(1);
        }
    };
    let cpp_compiler = match config.get_default_compiler().as_str() {
        "gcc" => "g++",
        "clang" => "clang++",
        _ => {
            log(LogLevel::Error, "Invalid default compiler");
            std::process::exit(1);
        }
    };
    let sample_cpp_config = format!("[build]\ncompiler = \"{}\"\n\n[[targets]]\nname = \"main\"\nsrc = \"./src/\"\ninclude_dir = \"./src/include/\"\ntype = \"exe\"\ncflags = \"-g -Wall -Wextra\"\nlibs = \"\"\ndeps = [\"\"]\n", cpp_compiler);

    let sample_c_config = format!("[build]\ncompiler = \"{}\"\n\n[[targets]]\nname = \"main\"\nsrc = \"./src/\"\ninclude_dir = \"./src/include/\"\ntype = \"exe\"\ncflags = \"-g -Wall -Wextra\"\nlibs = \"\"\ndeps = [\"\"]\n", c_compiler);

    let sample_config = match is_c {
        Some(true) => sample_c_config,
        Some(false) => sample_cpp_config,
        None => match config.get_default_language().as_str() {
            "c" => sample_c_config,
            "cpp" => sample_cpp_config,
            _ => {
                log(LogLevel::Error, "Invalid default language");
                std::process::exit(1);
            }
        },
    };

    config_file
        .write_all(sample_config.as_bytes())
        .unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not write to config file: {}", why),
            );
            std::process::exit(1);
        });

    let src_dir = project_name.to_owned() + "/src";
    let include_dir = project_name.to_owned() + "/src/include";

    //Create src and src/include directories
    if !Path::new(&src_dir).exists() {
        fs::create_dir(&src_dir).unwrap_or_else(|why| {
            log(LogLevel::Warn, &format!("Project name {}", project_name));
            log(
                LogLevel::Error,
                &format!("Could not create src directory: {}", why),
            );
            std::process::exit(1);
        });
    }
    if !Path::new(&include_dir).exists() {
        fs::create_dir(&include_dir).unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not create src/include directory: {}", why),
            );
            std::process::exit(1);
        });
    }

    //Create main.c or main.cpp
    let main_path: String;
    match is_c {
        Some(true) => main_path = src_dir.to_owned() + "/main.c",
        Some(false) => main_path = src_dir.to_owned() + "/main.cpp",
        None => match config.get_default_language().as_str() {
            "c" => main_path = src_dir.to_owned() + "/main.c",
            "cpp" => main_path = src_dir.to_owned() + "/main.cpp",
            _ => {
                log(LogLevel::Error, "Invalid default language");
                std::process::exit(1);
            }
        },
    }
    if !Path::new(&main_path).exists() {
        let mut main_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&main_path)
            .unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not create main.cpp: {}", why),
                );
                std::process::exit(1);
            });
        let c_sample_program =
            b"#include <stdio.h>\n\nint main() {\n\tprintf(\"Hello World!\\n\");\n\treturn 0;\n}";
        let cpp_sample_program = b"#include <iostream>\n\nint main() {\n\tstd::cout << \"Hello World!\" << std::endl;\n\treturn 0;\n}";
        match is_c {
            Some(true) => main_file.write_all(c_sample_program).unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not write to main.c: {}", why),
                );
                std::process::exit(1);
            }),
            Some(false) => main_file
                .write_all(cpp_sample_program)
                .unwrap_or_else(|why| {
                    log(
                        LogLevel::Error,
                        &format!("Could not write to main.cpp: {}", why),
                    );
                    std::process::exit(1);
                }),
            None => match config.get_default_language().as_str() {
                "c" => main_file.write_all(c_sample_program).unwrap_or_else(|why| {
                    log(
                        LogLevel::Error,
                        &format!("Could not write to main.c: {}", why),
                    );
                    std::process::exit(1);
                }),
                "cpp" => main_file
                    .write_all(cpp_sample_program)
                    .unwrap_or_else(|why| {
                        log(
                            LogLevel::Error,
                            &format!("Could not write to main.cpp: {}", why),
                        );
                        std::process::exit(1);
                    }),
                _ => {
                    log(LogLevel::Error, "Invalid default language");
                    std::process::exit(1);
                }
            },
        }
    }

    let gitignore_path = project_name.to_owned() + "/.gitignore";
    if !Path::new(&gitignore_path).exists() {
        let mut gitignore_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&gitignore_path)
            .unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not create .gitignore: {}", why),
                );
                std::process::exit(1);
            });
        gitignore_file
            .write_all(b".bld_cpp\ncompile_commands.json\n.cache\n")
            .unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not write to .gitignore: {}", why),
                );
                std::process::exit(1);
            });
    }

    let mut readme_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(project_name.to_owned() + "/README.md")
        .unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not create README.md: {}", why),
            );
            std::process::exit(1);
        });
    readme_file
        .write_all(format!("# {}", project_name).as_bytes())
        .unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not write to README.md: {}", why),
            );
            std::process::exit(1);
        });

    let mut license_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(project_name.to_owned() + "/LICENSE")
        .unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not create LICENSE: {}", why),
            );
            std::process::exit(1);
        });

    let license = config.get_license();
    if license.as_str() == "NONE" {
        license_file.write_all(b"No license").unwrap_or_else(|why| {
            log(
                LogLevel::Error,
                &format!("Could not write to LICENSE: {}", why),
            );
            std::process::exit(1);
        });
    } else {
        license_file
            .write_all(license.as_bytes())
            .unwrap_or_else(|why| {
                log(
                    LogLevel::Error,
                    &format!("Could not write to LICENSE: {}", why),
                );
                std::process::exit(1);
            });
    }

    log(
        LogLevel::Log,
        &format!("Project {} initialised", project_name),
    );
}

pub fn init_project(project_name: String, is_c: Option<bool>, config: GlobalConfig) {
    utils::log(utils::LogLevel::Log, "Initializing project...");
    init(&project_name, is_c, config);
    std::process::exit(0);
}

pub fn parse_config() -> (
    utils::BuildConfig,
    Vec<utils::TargetConfig>,
    Vec<utils::Package>,
) {
    #[cfg(target_os = "linux")]
    let (build_config, targets) = utils::parse_config("./config_linux.toml", true);
    #[cfg(target_os = "windows")]
    let (build_config, targets) = utils::parse_config("./config_win32.toml", true);
    #[cfg(target_os = "android")]
    let (build_config, targets) = utils::parse_config("./config_linux.toml", true);

    let mut num_exe = 0;
    let mut exe_target: Option<&utils::TargetConfig> = None;
    if targets.is_empty() {
        utils::log(utils::LogLevel::Error, "No targets in config");
        std::process::exit(1);
    } else {
        //Allow only one exe and set it as the exe_target
        for target in &targets {
            if target.typ == "exe" {
                num_exe += 1;
                exe_target = Some(target);
            } else if target.typ == "dll" {
                // Check if the dll target name starts with lib
                if !target.name.starts_with("lib") {
                    utils::log(
                        utils::LogLevel::Warn,
                        "Dynamic library target name must start with lib",
                    );
                    utils::log(
                        utils::LogLevel::Warn,
                        format!(
                            "Consider renaming \"{}\" to \"lib{}\"",
                            target.name, target.name
                        )
                        .as_str(),
                    );
                    utils::log(
                        utils::LogLevel::Log,
                        "Libraries are prefixed with \"lib\" and the lib is replaced by -l when linking",
                    );
                    utils::log(
                        utils::LogLevel::Log,
                        "For example, libmylib.so becomes -lmylib",
                    );
                }
            }
        }
    }

    if num_exe != 1 || exe_target.is_none() {
        utils::log(
            utils::LogLevel::Error,
            "Exactly one executable target must be specified",
        );
        std::process::exit(1);
    }

    #[cfg(target_os = "linux")]
    let packages = utils::Package::parse_packages("./config_linux.toml");
    #[cfg(target_os = "android")]
    let packages = utils::Package::parse_packages("./config_linux.toml");
    #[cfg(target_os = "windows")]
    let packages = utils::Package::parse_packages("./config_win32.toml");

    (build_config, targets, packages)
}

pub fn pre_gen_cc() {
    if !Path::new("./compile_commands.json").exists() {
        fs::File::create(Path::new("./compile_commands.json")).unwrap();
    } else {
        fs::remove_file(Path::new("./compile_commands.json")).unwrap();
        fs::File::create(Path::new("./compile_commands.json")).unwrap();
    }
}

pub fn pre_gen_vsc() {
    if !Path::new("./.vscode").exists() {
        fs::create_dir(Path::new("./.vscode")).unwrap();
    }

    if !Path::new("./.vscode/c_cpp_properties.json").exists() {
        fs::File::create(Path::new("./.vscode/c_cpp_properties.json")).unwrap();
    } else {
        fs::remove_file(Path::new("./.vscode/c_cpp_properties.json")).unwrap();
        fs::File::create(Path::new("./.vscode/c_cpp_properties.json")).unwrap();
    }
}

pub fn clean_packages_wrapper(packages: &Vec<utils::Package>) {
    utils::log(utils::LogLevel::Log, "Cleaning packages...");
    clean_packages(packages);
}

pub fn update_packages(packages: &Vec<utils::Package>) {
    utils::log(utils::LogLevel::Log, "Updating packages...");
    for package in packages {
        package.update();
    }
}

pub fn restore_packages(packages: &Vec<utils::Package>) {
    utils::log(utils::LogLevel::Log, "Restoring packages...");
    for package in packages {
        package.restore();
    }
}
