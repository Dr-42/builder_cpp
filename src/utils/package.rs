use std::{path::Path, process::Command};

use super::configs::{parse_config, BuildConfig, TargetConfig};
use super::log::{log, LogLevel};

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
    pub fn new(
        name: String,
        repo: String,
        branch: String,
        build_config: BuildConfig,
        target_configs: Vec<TargetConfig>,
    ) -> Self {
        Self {
            name,
            repo,
            branch,
            build_config,
            target_configs,
        }
    }

    pub fn obj_dir() -> &'static str {
        #[cfg(target_os = "linux")]
        let obj_dir = "./.bld_cpp/obj_linux";
        #[cfg(target_os = "android")]
        let obj_dir = "./.bld_cpp/obj_android";
        #[cfg(target_os = "windows")]
        let obj_dir = "./.bld_cpp/obj_win32";

        obj_dir
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
            log(
                LogLevel::Log,
                &format!("Successfully updated package: {}", self.name),
            );
            log(
                LogLevel::Log,
                &format!("Output: {}", String::from_utf8_lossy(&com.stdout))
                    .replace('\r', "")
                    .replace('\n', ""),
            );
        } else {
            log(
                LogLevel::Error,
                &format!(
                    "Failed to update package: {}",
                    String::from_utf8_lossy(&com.stderr)
                ),
            );
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
                log(
                    LogLevel::Error,
                    &format!("Failed to restore package: {}", e),
                );
                std::process::exit(1);
            });
        if com.status.success() {
            log(
                LogLevel::Log,
                &format!("Successfully restored package: {}", self.name),
            );
            log(
                LogLevel::Log,
                &format!("Output: {}", String::from_utf8_lossy(&com.stdout))
                    .replace('\r', "")
                    .replace('\n', ""),
            );
        } else {
            log(
                LogLevel::Error,
                &format!(
                    "Failed to restore package: {}",
                    String::from_utf8_lossy(&com.stderr)
                ),
            );
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
            cstandard: Some("c11".to_string()),
            cppstandard: Some("c++17".to_string()),
            pre_build: None,
            post_build: None,
        };
        let mut target_configs = Vec::new();

        //parse the root toml file
        let (build_config_toml, _) = parse_config(path, false);
        for package in build_config_toml.packages {
            let deets = package.split_whitespace().collect::<Vec<&str>>();
            if deets.len() != 2 {
                log(
                    LogLevel::Error,
                    "Packages must be in the form of \"<git_repo> <branch>\"",
                );
                std::process::exit(1);
            }
            repo = deets[0].to_string().replace(',', "");
            branch = deets[1].to_string();

            name = repo.split('/').collect::<Vec<&str>>()[1].to_string();
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
                log(
                    LogLevel::Log,
                    &format!("Cloning {} into {}", repo, source_dir),
                );
                let repo_https = format!("https://github.com/{}", repo);
                let mut cmd = Command::new("git");
                cmd.arg("clone")
                    .arg("--branch")
                    .arg(&branch)
                    .arg(&repo_https)
                    .arg(&source_dir);
                let output = cmd.output().expect("Failed to execute git clone");
                if !output.status.success() {
                    log(
                        LogLevel::Error,
                        &format!(
                            "Failed to clone {} branch {} into {}",
                            repo, branch, source_dir
                        ),
                    );
                    std::process::exit(1);
                }
            }
            #[cfg(target_os = "linux")]
            let pkg_toml = format!("{}/config_linux.toml", source_dir).replace("//", "/");
            #[cfg(target_os = "android")]
            let pkg_toml = format!("{}/config_linux.toml", source_dir).replace("//", "/");
            #[cfg(target_os = "windows")]
            let pkg_toml = format!("{}/config_win32.toml", source_dir).replace("//", "/");

            let (pkg_bld_config_toml, pkg_targets_toml) = parse_config(&pkg_toml, false);
            log(LogLevel::Info, &format!("Parsed {}", pkg_toml));

            if !pkg_bld_config_toml.packages.is_empty() {
                for foreign_package in Package::parse_packages(&pkg_toml) {
                    packages.push(foreign_package);
                }
            }

            build_config = pkg_bld_config_toml;
            build_config.compiler = build_config_toml.compiler.clone();
            if !Path::new(Package::obj_dir()).exists() {
                let cmd = Command::new("mkdir")
                    .arg("-p")
                    .arg(Package::obj_dir())
                    .output();
                if cmd.is_err() {
                    log(
                        LogLevel::Error,
                        &format!("Failed to create {}", Package::obj_dir()),
                    );
                    std::process::exit(1);
                }
                log(LogLevel::Info, &format!("Created {}", Package::obj_dir()));
            }

            let tgt_configs = pkg_targets_toml;
            for mut tgt in tgt_configs {
                if tgt.typ != "dll" {
                    continue;
                }
                tgt.src = format!("{}/{}", source_dir, tgt.src)
                    .replace('\\', "/")
                    .replace("/./", "/")
                    .replace("//", "/");
                let old_inc_dir = tgt.include_dir.clone();
                tgt.include_dir = format!("./.bld_cpp/includes/{}", name)
                    .replace('\\', "/")
                    .replace("/./", "/")
                    .replace("//", "/");
                if !Path::new(&tgt.include_dir).exists() {
                    let cmd = Command::new("mkdir")
                        .arg("-p")
                        .arg(&tgt.include_dir)
                        .output();
                    if cmd.is_err() {
                        log(
                            LogLevel::Error,
                            &format!("Failed to create {}", tgt.include_dir),
                        );
                        std::process::exit(1);
                    }
                    log(LogLevel::Info, &format!("Created {}", tgt.include_dir));
                    let mut cm = String::new();
                    cm.push_str("cp -r ");
                    cm.push_str(
                        &format!("{}/{}/* ", source_dir, old_inc_dir)
                            .replace('\\', "/")
                            .replace("/./", "/")
                            .replace("//", "/"),
                    );
                    cm.push_str(&tgt.include_dir);
                    cm.push_str("/ ");
                    let cmd = Command::new("sh").arg("-c").arg(&cm).output();
                    if cmd.is_err() {
                        log(
                            LogLevel::Error,
                            &format!("Failed to create {}", tgt.include_dir),
                        );
                        std::process::exit(1);
                    }
                }
                target_configs.push(tgt);
            }
        }

        packages.push(Package::new(
            name,
            repo,
            branch,
            build_config,
            target_configs,
        ));
        packages.sort_by_key(|a| a.name.clone());
        packages.dedup_by_key(|a| a.name.clone());
        packages
    }
}
