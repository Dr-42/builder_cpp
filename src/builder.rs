//! This module contains the buiild related functions

use crate::hasher;
use crate::utils::{self, log, BuildConfig, LogLevel, Package, TargetConfig};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

static BUILD_DIR: &str = ".bld_cpp/bin";
#[cfg(target_os = "windows")]
static OBJ_DIR: &str = ".bld_cpp/obj_win32";
#[cfg(target_os = "linux")]
static OBJ_DIR: &str = ".bld_cpp/obj_linux";
#[cfg(target_os = "android")]
static OBJ_DIR: &str = ".bld_cpp/obj_linux";

//Represents a target
pub struct Target<'a> {
    srcs: Vec<Src>,
    build_config: &'a BuildConfig,
    target_config: &'a TargetConfig,
    dependant_includes: HashMap<String, Vec<String>>,
    pub bin_path: String,
    hash_file_path: String,
    path_hash: HashMap<String, String>,
    dependant_libs: Vec<Target<'a>>,
    packages: &'a Vec<Package>,
}

//Represents a source file
//A single C or Cpp file
struct Src {
    path: String,
    name: String,
    obj_name: String,
    bin_path: String,
    dependant_includes: Vec<String>,
}

impl<'a> Target<'a> {
    /// Creates a new target
    /// # Arguments
    /// * `build_config` - Build config
    /// * `target_config` - Target config
    /// * `targets` - All targets
    /// * `packages` - All packages
    pub fn new(
        build_config: &'a BuildConfig,
        target_config: &'a TargetConfig,
        targets: &'a Vec<TargetConfig>,
        packages: &'a Vec<Package>,
    ) -> Self {
        let srcs = Vec::new();
        let dependant_includes: HashMap<String, Vec<String>> = HashMap::new();

        let mut bin_path = String::new();
        bin_path.push_str(BUILD_DIR);
        bin_path.push('/');
        bin_path.push_str(&target_config.name);
        #[cfg(target_os = "windows")]
        if target_config.typ == "exe" {
            bin_path.push_str(".exe");
        } else if target_config.typ == "dll" {
            bin_path.push_str(".dll");
        }
        #[cfg(target_os = "linux")]
        if target_config.typ == "exe" {
            bin_path.push_str("");
        } else if target_config.typ == "dll" {
            bin_path.push_str(".so");
        }
        #[cfg(target_os = "android")]
        if target_config.typ == "exe" {
            bin_path.push_str("");
        } else if target_config.typ == "dll" {
            bin_path.push_str(".so");
        }

        #[cfg(target_os = "windows")]
        let hash_file_path = format!(".bld_cpp/{}.win32.hash", &target_config.name);
        #[cfg(target_os = "linux")]
        let hash_file_path = format!(".bld_cpp/{}.linux.hash", &target_config.name);
        #[cfg(target_os = "android")]
        let hash_file_path = format!(".bld_cpp/{}.linux.hash", &target_config.name);

        let path_hash = hasher::load_hashes_from_file(&hash_file_path);
        let mut dependant_libs = Vec::new();
        for dependant_lib in &target_config.deps {
            for target in targets {
                if target.name == *dependant_lib {
                    dependant_libs.push(Target::new(build_config, target, targets, packages));
                }
            }
        }
        for dep_lib in &dependant_libs {
            if dep_lib.target_config.typ != "dll" {
                utils::log(LogLevel::Error, "Can add only dlls as dependant libs");
                utils::log(
                    LogLevel::Error,
                    &format!("Target: {} is not a dll", dep_lib.target_config.name),
                );
                utils::log(
                    LogLevel::Error,
                    &format!(
                        "Target: {} is a {}",
                        dep_lib.target_config.name, dep_lib.target_config.typ
                    ),
                );
                std::process::exit(1);
            } else {
                utils::log(
                    LogLevel::Info,
                    &format!("Adding dependant lib: {}", dep_lib.target_config.name),
                );
            }
            if !dep_lib.target_config.name.starts_with("lib") {
                utils::log(LogLevel::Error, "Dependant lib name must start with lib");
                utils::log(
                    LogLevel::Error,
                    &format!(
                        "Target: {} does not start with lib",
                        dep_lib.target_config.name
                    ),
                );
                std::process::exit(1);
            }
        }
        if target_config.deps.len() > dependant_libs.len() + packages.len() {
            utils::log(LogLevel::Error, "Dependant libs not found");
            utils::log(
                LogLevel::Error,
                &format!("Dependant libs: {:?}", target_config.deps),
            );
            utils::log(
                LogLevel::Error,
                &format!(
                    "Found libs: {:?}",
                    targets
                        .iter()
                        .map(|x| {
                            if x.typ == "dll" {
                                x.name.clone()
                            } else {
                                "".to_string()
                            }
                        })
                        .collect::<Vec<String>>()
                        .into_iter()
                        .filter(|x| !x.is_empty())
                        .collect::<Vec<String>>()
                ),
            );
            std::process::exit(1);
        }

        let mut target = Target::<'a> {
            srcs,
            build_config,
            target_config,
            dependant_includes,
            bin_path,
            path_hash,
            hash_file_path,
            dependant_libs,
            packages,
        };
        target.get_srcs(&target_config.src, target_config);
        target
    }

    /// Builds the target
    /// # Arguments
    /// * `gen_cc` - Generate compile_commands.json
    pub fn build(&mut self, gen_cc: bool) {
        if !Path::new(".bld_cpp").exists() {
            std::fs::create_dir(".bld_cpp").unwrap_or_else(|why| {
                utils::log(
                    LogLevel::Error,
                    &format!("Couldn't create .bld_cpp directory: {}", why),
                );
                std::process::exit(1);
            });
        }
        for pkg in self.packages {
            for target in &pkg.target_configs {
                let empty: Vec<Package> = Vec::new();
                if target.typ == "dll" {
                    let mut pkg_tgt =
                        Target::new(&pkg.build_config, target, &pkg.target_configs, &empty);
                    pkg_tgt.build(gen_cc);
                }
            }
        }
        let mut to_link: bool = false;
        let mut link_causer: Vec<&str> = Vec::new();
        let mut srcs_needed = 0;
        let total_srcs = self.srcs.len();
        let mut src_ccs = Vec::new();
        for src in &self.srcs {
            let (to_build, _) = src.to_build(&self.path_hash);
            log(LogLevel::Debug, &format!("{}: {}", src.path, to_build));
            if to_build {
                to_link = true;
                link_causer.push(&src.path);
                srcs_needed += 1;
            }
            if gen_cc {
                src_ccs.push(self.gen_cc(src));
            }
        }
        if gen_cc {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open("./compile_commands.json")
                .unwrap();
            for src_cc in src_ccs {
                if let Err(e) = writeln!(file, "{},", src_cc) {
                    eprintln!("Couldn't write to file: {}", e);
                }
            }
        }
        if to_link {
            log(
                LogLevel::Log,
                &format!("Compiling Target: {}", &self.target_config.name),
            );
            log(
                LogLevel::Log,
                &format!(
                    "\t {} of {} source files have to be compiled",
                    srcs_needed, total_srcs
                ),
            );
            if !Path::new(OBJ_DIR).exists() {
                fs::create_dir(OBJ_DIR).unwrap_or_else(|why| {
                    log(
                        LogLevel::Error,
                        &format!("Couldn't create obj dir: {}", why),
                    );
                });
            }
        } else {
            log(
                LogLevel::Log,
                &format!("Target: {} is up to date", &self.target_config.name),
            );
            return;
        }
        let progress_bar = Arc::new(Mutex::new(ProgressBar::new(srcs_needed as u64)));

        let num_complete = Arc::new(Mutex::new(0));
        let src_hash_to_update = Arc::new(Mutex::new(Vec::new()));
        let warns = Arc::new(Mutex::new(Vec::new()));
        self.srcs.par_iter().for_each(|src| {
            let (to_build, _message) = src.to_build(&self.path_hash);
            log(LogLevel::Debug, &format!("{}: {}", src.path, to_build));
            if to_build {
                let warn = src.build(self.build_config, self.target_config, &self.dependant_libs);
                if let Some(warn) = warn {
                    warns.lock().unwrap().push(warn);
                }
                src_hash_to_update.lock().unwrap().push(src);
                log(LogLevel::Info, &format!("Compiled: {}", src.path));
                let log_level = std::env::var("BUILDER_CPP_LOG_LEVEL").unwrap_or("".to_string());
                if !(log_level == "Info" || log_level == "Debug") {
                    let mut num_complete = num_complete.lock().unwrap();
                    *num_complete += 1;
                    let progress_bar = progress_bar.lock().unwrap();
                    let template = format!(
                        "    {}{}",
                        "Compiling :".cyan(),
                        "[{bar:40.}] {pos}/{len} ({percent}%) {msg}[{elapsed_precise}] "
                    );
                    progress_bar.set_style(
                        ProgressStyle::with_template(&template)
                            .unwrap()
                            .progress_chars("=>-"),
                    );
                    progress_bar.inc(1);
                }
            }
        });
        let warns = warns.lock().unwrap();
        if warns.len() > 0 {
            log(LogLevel::Warn, "Warnings emitted during build:");
            for warn in warns.iter() {
                log(LogLevel::Warn, &format!("\t{}", warn));
            }
        }

        for src in src_hash_to_update.lock().unwrap().iter() {
            hasher::save_hash(&src.path, &mut self.path_hash);
        }
        if to_link {
            log(LogLevel::Log, "Linking: Since source files were compiled");
            for src in link_causer {
                log(LogLevel::Info, &format!("\tFile: {}", &src));
            }
            for src in &self.srcs {
                for include in &src.dependant_includes {
                    hasher::save_hash(include, &mut self.path_hash);
                }
            }
            hasher::save_hashes_to_file(&self.hash_file_path, &self.path_hash);
            self.link(&self.dependant_libs);
        }
    }

    /// Links the target
    /// # Arguments
    /// * `dep_targets` - The targets that this target depends on
    pub fn link(&self, dep_targets: &Vec<Target>) {
        let mut objs = Vec::new();
        if !Path::new(BUILD_DIR).exists() {
            let cmd = format!("mkdir -p {}", BUILD_DIR);
            let output = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .expect("failed to execute process");
            if !output.status.success() {
                log(
                    LogLevel::Error,
                    &format!(
                        "Couldn't create build dir: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                );
            }
        }
        for src in &self.srcs {
            objs.push(&src.obj_name);
        }

        let mut cmd = String::new();
        cmd.push_str(&self.build_config.compiler);
        cmd.push_str(" -o ");
        cmd.push_str(&self.bin_path);
        if self.target_config.typ == "dll" {
            cmd.push_str(" -shared ");
        }

        for obj in objs {
            cmd.push(' ');
            cmd.push_str(obj);
        }
        cmd.push(' ');
        cmd.push_str(&self.target_config.cflags);
        cmd.push(' ');
        for dep_target in dep_targets {
            cmd.push_str(" -I");
            cmd.push_str(&dep_target.target_config.include_dir);
            cmd.push(' ');

            let lib_name = dep_target.target_config.name.clone();
            let lib_name = lib_name.replace("lib", "-l");
            cmd.push_str(&lib_name);
            cmd.push(' ');
        }

        for package in self.packages {
            for target in &package.target_configs {
                cmd.push_str(" -I");
                cmd.push_str(&target.include_dir);
                cmd.push(' ');

                let lib_name = target.name.clone();
                let lib_name = lib_name.replace("lib", "-l");
                cmd.push_str(&lib_name);
                cmd.push(' ');
            }
        }

        if self.packages.len() + self.dependant_libs.len() > 0 {
            cmd.push_str("-L");
            cmd.push_str(BUILD_DIR);
            #[cfg(target_os = "linux") | cfg(target_os = "android")]
            cmd.push_str(" -Wl,-rpath,\'$ORIGIN\' ");
            cmd.push(' ');
        }
        cmd.push_str(&self.target_config.libs);

        log(
            LogLevel::Info,
            &format!("Linking target: {}", &self.target_config.name),
        );
        log(LogLevel::Info, &format!("  Command: {}", &cmd));
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()
            .expect("failed to execute process");
        if output.status.success() {
            log(LogLevel::Info, "  Linking successful");
            hasher::save_hashes_to_file(&self.hash_file_path, &self.path_hash);
        } else {
            log(LogLevel::Error, "  Linking failed");
            log(LogLevel::Error, &format!("  Command: {}", &cmd));
            log(
                LogLevel::Error,
                &format!("  Error: {}", String::from_utf8_lossy(&output.stderr)),
            );
            std::process::exit(1);
        }
    }

    /// Generates the compile_commands.json file for a Src
    fn gen_cc(&self, src: &Src) -> String {
        let mut cc = String::new();
        cc.push_str("{\n");
        if self.build_config.compiler == "clang++" || self.build_config.compiler == "g++" {
            cc.push_str("\t\"command\": \"c++");
        } else if self.build_config.compiler == "clang" || self.build_config.compiler == "gcc" {
            cc.push_str("\t\"command\": \"cc");
        } else {
            log(
                LogLevel::Error,
                &format!("Compiler: {} is not supported", &self.build_config.compiler),
            );
            log(
                LogLevel::Error,
                "Supported compilers: clang++, g++, clang, gcc",
            );
            std::process::exit(1);
        }
        cc.push_str(" -c -o ");
        cc.push_str(&src.obj_name);
        cc.push_str(" -I");
        cc.push_str(&self.target_config.include_dir);

        for lib in &self.dependant_libs {
            cc.push_str(" -I");
            cc.push_str(&lib.target_config.include_dir);
        }
        for pack in self.packages {
            for tgtg in &pack.target_configs {
                cc.push_str(" -I");
                cc.push_str(&tgtg.include_dir);
            }
        }

        cc.push(' ');
        let cflags = &self.target_config.cflags;

        let subcmds = cflags.split('`').collect::<Vec<&str>>();
        // Take even entries are non-subcmds and odd entries are subcmds
        let (subcmds, non_subcmds): (Vec<String>, String) = subcmds.iter().enumerate().fold(
            (Vec::new(), String::new()),
            |(mut subcmds, mut non_subcmds), (i, subcmd)| {
                if i % 2 != 0 {
                    subcmds.push(subcmd.to_string());
                } else {
                    non_subcmds.push_str(subcmd);
                    non_subcmds.push(' ');
                }
                (subcmds, non_subcmds)
            },
        );

        cc.push_str(&non_subcmds);

        for subcmd in subcmds {
            let cmd_output = Command::new("sh")
                .arg("-c")
                .arg(&subcmd)
                .output()
                .expect("failed to execute process");
            if cmd_output.status.success() {
                let stdout = String::from_utf8_lossy(&cmd_output.stdout);
                let stdout = stdout.replace('\n', " ");
                cc.push_str(&stdout);
            } else {
                let stderr = String::from_utf8_lossy(&cmd_output.stderr);
                log(
                    LogLevel::Error,
                    &format!("Failed to execute subcmd: {}", &subcmd),
                );
                log(LogLevel::Error, &format!("  Stderr: {}", stderr));
                std::process::exit(1);
            }
        }

        #[cfg(target_os = "linux")]
        if self.target_config.typ == "dll" {
            cc.push_str("-fPIC ");
        }
        #[cfg(target_os = "android")]
        if self.target_config.typ == "dll" {
            cc.push_str("-fPIC ");
        }

        cc.push_str(&src.path);
        cc.push_str("\",\n");
        let mut dirent = String::new();
        dirent.push_str("\t\"directory\": \"");
        dirent.push_str(
            &std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/"),
        );
        dirent.push_str("\",\n");
        let dirent = dirent.replace('/', "\\\\").replace("\\\\.\\\\", "\\\\");
        cc.push_str(&dirent);
        let mut fileent = String::new();
        fileent.push_str("\t\"file\": \"");
        fileent.push_str(
            &std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/"),
        );
        fileent.push('/');
        fileent.push_str(&src.path);
        fileent.push('\"');
        let fileent = fileent.replace('/', "\\\\").replace("\\\\.\\\\", "\\\\");

        cc.push_str(&fileent);

        cc.push_str("\n}");
        #[cfg(target_os = "linux")]
        return cc.replace("\\\\", "/");
        #[cfg(target_os = "android")]
        return cc.replace("\\\\", "/");
        #[cfg(target_os = "windows")]
        return cc;
    }
    //returns a vector of source files in the given root path
    fn get_srcs(&mut self, root_path: &str, _target_config: &'a TargetConfig) -> Vec<Src> {
        let root_dir = PathBuf::from(root_path);
        let mut srcs: Vec<Src> = Vec::new();
        let root_entries = std::fs::read_dir(root_dir).unwrap_or_else(|_| {
            log(
                LogLevel::Error,
                &format!("Could not read directory: {}", root_path),
            );
            std::process::exit(1);
        });
        for entry in root_entries {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                let path = entry.path().to_str().unwrap().to_string();
                srcs.append(&mut self.get_srcs(&path, _target_config));
            } else {
                if !entry.path().to_str().unwrap().ends_with(".cpp")
                    && !entry.path().to_str().unwrap().ends_with(".c")
                {
                    continue;
                }
                let path = entry
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string()
                    .replace('\\', "/");
                self.add_src(path);
            }
        }
        srcs
    }

    //adds a source file to the target
    fn add_src(&mut self, path: String) {
        let name = Target::get_src_name(&path);
        let obj_name = self.get_src_obj_name(&name);
        let dependant_includes = self.get_dependant_includes(&path);
        let bin_path = self.bin_path.clone();
        self.srcs
            .push(Src::new(path, name, obj_name, bin_path, dependant_includes));
    }

    //returns the file name without the extension from the path
    fn get_src_name(path: &str) -> String {
        let path_buf = PathBuf::from(path);
        let file_name = path_buf.file_name().unwrap().to_str().unwrap();
        let name = file_name.split('.').next().unwrap();
        name.to_string()
    }

    //return the object file name for the given source file
    fn get_src_obj_name(&self, src_name: &str) -> String {
        let mut obj_name = String::new();
        obj_name.push_str(OBJ_DIR);
        obj_name.push('/');
        obj_name.push_str(&self.target_config.name);
        obj_name.push_str(src_name);
        obj_name.push_str(".o");
        obj_name
    }

    //returns a vector of .h or .hpp files the given C/C++ depends on
    fn get_dependant_includes(&mut self, path: &str) -> Vec<String> {
        let mut result = Vec::new();
        let include_substrings = self.get_include_substrings(path).unwrap_or_else(|| {
            log(
                LogLevel::Error,
                &format!("Failed to get include substrings for file: {}", path),
            );
            log(
                LogLevel::Error,
                &format!(
                    "File included from: {:?}",
                    self.dependant_includes.get(path)
                ),
            );
            std::process::exit(1);
        });
        if include_substrings.is_empty() {
            return result;
        }
        for include_substring in include_substrings {
            let dep_path = format!("{}/{}", &self.target_config.include_dir, &include_substring);
            if self.dependant_includes.contains_key(&dep_path) {
                continue;
            }
            result.append(&mut self.get_dependant_includes(&dep_path));
            result.push(dep_path);
            self.dependant_includes
                .insert(include_substring, result.clone());
        }

        result.into_iter().unique().collect()
    }

    //returns a vector of strings that are the include substrings
    //of the given C/C++ file as variaible path
    fn get_include_substrings(&self, path: &str) -> Option<Vec<String>> {
        let file = std::fs::File::open(path);
        if file.is_err() {
            return None;
        }
        let mut file = file.unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let lines = buf.lines();
        let mut include_substrings = Vec::new();
        for line in lines {
            if line.starts_with("#include \"") {
                let include_path = line.split('\"').nth(1).unwrap().to_owned();
                include_substrings.push(include_path);
            }
        }
        Some(include_substrings)
    }
}

impl Src {
    //Creates a new source file
    fn new(
        path: String,
        name: String,
        obj_name: String,
        bin_path: String,
        dependant_includes: Vec<String>,
    ) -> Self {
        Self {
            path,
            name,
            obj_name,
            bin_path,
            dependant_includes,
        }
    }

    //returns a tuple of a bool and a string
    //the bool is true if the source file needs to be built
    //the string is the reason the source file needs to be built
    fn to_build(&self, path_hash: &HashMap<String, String>) -> (bool, String) {
        if !Path::new(&self.bin_path).exists() {
            let result = (true, format!("\tBinary does not exist: {}", &self.bin_path));
            return result;
        }

        if hasher::is_file_changed(&self.path, path_hash) {
            let result = (true, format!("\tSource file has changed: {}", &self.path));
            return result;
        }
        for dependant_include in &self.dependant_includes {
            if hasher::is_file_changed(&dependant_include.clone(), path_hash) {
                let result = (
                    true,
                    format!(
                        "\tSource file: {} depends on changed include file: {}",
                        &self.path, &dependant_include
                    ),
                );
                return result;
            }
        }

        (
            false,
            format!("Source file: {} does not need to be built", &self.path),
        )
    }

    //builds the source file
    fn build(
        &self,
        build_config: &BuildConfig,
        target_config: &TargetConfig,
        dependant_libs: &Vec<Target>,
    ) -> Option<String> {
        let mut cmd = String::new();
        cmd.push_str(&build_config.compiler);
        cmd.push_str(" -c ");
        cmd.push_str(&self.path);
        cmd.push_str(" -o ");
        cmd.push_str(&self.obj_name);
        cmd.push_str(" -I");
        cmd.push_str(&target_config.include_dir);
        cmd.push(' ');

        for dependant_lib in dependant_libs {
            cmd.push_str("-I");
            cmd.push_str(dependant_lib.target_config.include_dir.as_str());
            cmd.push(' ');
        }

        if !build_config.packages.is_empty() {
            for package in &build_config.packages {
                cmd.push_str("-I");
                cmd.push_str(&format!(
                    ".bld_cpp/includes/{} ",
                    &package
                        .split_whitespace()
                        .next()
                        .unwrap()
                        .split('/')
                        .last()
                        .unwrap()
                        .replace(',', "")
                ));
                cmd.push(' ');
            }
        }

        cmd.push_str(&target_config.cflags);

        if target_config.typ == "dll" {
            cmd.push_str(" -fPIC");
        }

        log(LogLevel::Info, &format!("Building: {}", &self.name));
        log(LogLevel::Info, &format!("  Command: {}", &cmd));
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()
            .expect("failed to execute process");
        if output.status.success() {
            log(LogLevel::Info, &format!("  Success: {}", &self.name));
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.len() > 0 {
                log(LogLevel::Info, &format!("  Stdout: {}", stdout));
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.len() > 0 {
                return Some(stderr.to_string());
            }
            None
        } else {
            log(LogLevel::Error, &format!("  Error: {}", &self.name));
            log(LogLevel::Error, &format!("  Command: {}", &cmd));
            log(
                LogLevel::Error,
                &format!("  Stdout: {}", String::from_utf8_lossy(&output.stdout)),
            );
            log(
                LogLevel::Error,
                &format!("  Stderr: {}", String::from_utf8_lossy(&output.stderr)),
            );
            std::process::exit(1);
        }
    }
}
