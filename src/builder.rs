use std::fs;
use std::sync::{Arc, Mutex};
use crate::utils::{BuildConfig, TargetConfig, Package, log, LogLevel, self};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::process::Command;
use itertools::Itertools;
use std::collections::HashMap;
use crate::hasher;
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use colored::Colorize;

//Represents a target
pub struct Target<'a> {
    pub srcs: Vec<Src>,
    pub build_config: &'a BuildConfig,
    pub target_config: &'a TargetConfig,
    dependant_includes: HashMap<String, Vec<String>>,
    pub bin_path: String,
    pub hash_file_path: String,
    path_hash: HashMap<String, String>,
    pub dependant_libs: Vec<Target<'a>>,
    pub packages: &'a Vec<Package>,
}

//Represents a source file
//A single C or Cpp file
pub struct Src {
    pub path: String,
    pub name: String,
    pub obj_name: String,
    pub dependant_includes: Vec<String>,
}

impl<'a> Target<'a> {
    pub fn new(build_config: &'a BuildConfig, target_config: &'a TargetConfig, targets: &'a Vec<TargetConfig>, packages: &'a Vec<Package>) -> Self {
        let srcs = Vec::new();
        let dependant_includes: HashMap<String, Vec<String>> = HashMap::new();
        
        let mut bin_path = String::new();
        bin_path.push_str(&build_config.build_dir);
        bin_path.push_str("/");
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

        #[cfg(target_os = "windows")]
        let hash_file_path = format!(".bld_cpp/{}.win32.hash", &target_config.name);
        #[cfg(target_os = "linux")]
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
                utils::log(LogLevel::Error, &format!("Target: {} is not a dll", dep_lib.target_config.name));
                utils::log(LogLevel::Error, &format!("Target: {} is a {}", dep_lib.target_config.name, dep_lib.target_config.typ));
                std::process::exit(1);
            }
            else {
                utils::log(LogLevel::Info, &format!("Adding dependant lib: {}", dep_lib.target_config.name));
            }
            if !dep_lib.target_config.name.starts_with("lib") {
                utils::log(LogLevel::Error, "Dependant lib name must start with lib");
                utils::log(LogLevel::Error, &format!("Target: {} does not start with lib", dep_lib.target_config.name));
                std::process::exit(1);
            }
        }
        if target_config.deps.len() > dependant_libs.len() + packages.len() {
            utils::log(LogLevel::Error, "Dependant libs not found");
            utils::log(LogLevel::Error, &format!("Dependant libs: {:?}", target_config.deps));
            utils::log(LogLevel::Error, &format!("Found libs: {:?}", targets.iter().map(|x| {
                if x.typ == "dll" {
                    x.name.clone()
                } else {
                    "".to_string()
                }
            }).collect::<Vec<String>>().into_iter().filter(|x| x != "").collect::<Vec<String>>()));
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

    pub fn build(&mut self, gen_cc: bool ) {
        for pkg in self.packages {
            for target in &pkg.target_configs {
                let empty: Vec<Package> = Vec::new();
                if target.typ == "dll" {
                    let mut pkg_tgt = Target::new(&pkg.build_config, &target, &pkg.target_configs, &empty);
                    pkg_tgt.build(gen_cc);
                }
            }
        }
        let mut to_link : bool = false;
        let mut link_causer : Vec<&str> = Vec::new();
        let mut srcs_needed = 0;
        let total_srcs = self.srcs.len();
        let mut src_ccs = Vec::new();
        for src in &self.srcs {
            let (to_build, _) = src.to_build(&self.path_hash);
            if to_build {
                hasher::save_hash(&src.path, &mut self.path_hash);
                to_link = true;
                link_causer.push(&src.path);
                srcs_needed += 1;
            }
            if gen_cc {
                src_ccs.push(self.gen_cc(&src));
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
            log(LogLevel::Log, &format!("Compiling Target: {}", &self.target_config.name));
            log(LogLevel::Log, &format!("\t {} of {} source files have to be compiled", srcs_needed, total_srcs));
            if !Path::new(&self.build_config.obj_dir).exists() {
                fs::create_dir(&self.build_config.obj_dir).unwrap_or_else(|why| {
                    log(LogLevel::Error, &format!("Couldn't create obj dir: {}", why));
                });
            }
        } else {
            log(LogLevel::Log, &format!("Target: {} is up to date", &self.target_config.name));
            return;
        }
        let progress_bar = Arc::new(Mutex::new(ProgressBar::new(srcs_needed as u64)
        ));

        let num_complete = Arc::new(Mutex::new(0));
        self.srcs.par_iter().for_each(|src| {
            let (to_build, _message) = src.to_build(&self.path_hash);
            if to_build {
                src.build(self.build_config, self.target_config, &self.dependant_libs);
                let log_level = std::env::var("BUILDER_CPP_LOG_LEVEL").unwrap_or("".to_string());
                if !(log_level == "Info" || log_level == "Debug"){
                    let mut num_complete = num_complete.lock().unwrap();
                    *num_complete += 1;
                    let progress_bar = progress_bar.lock().unwrap();
                    let template = format!("    {}{}", "Compiling :".cyan(), "[{bar:40.white/white}] {pos}/{len} ({percent}%) {msg}[{elapsed_precise}] ");
                    progress_bar.set_style(ProgressStyle::with_template(&template)
                    .unwrap()
                    .progress_chars("=>-"));
                    progress_bar.inc(1);
                }
            }
        });
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

    pub fn link(&self, dep_targets: &Vec<Target>) {
        let mut objs = Vec::new();
        if !Path::new(&self.build_config.build_dir).exists() {
            let cmd = format!("mkdir -p {}", &self.build_config.build_dir);
            let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .expect("failed to execute process");
            if !output.status.success() {
                log(LogLevel::Error, &format!("Couldn't create build dir: {}", String::from_utf8_lossy(&output.stderr)));
            }
            else {
                log(LogLevel::Log, &format!("Created build dir: {}", &self.build_config.build_dir));
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
            cmd.push_str(" ");
            cmd.push_str(obj);
        }
        cmd.push_str(" ");
        cmd.push_str(&self.target_config.cflags);
        cmd.push_str(" ");
        for dep_target in dep_targets {
            cmd.push_str(" -I");
            cmd.push_str(&dep_target.target_config.include_dir);
            cmd.push_str(" ");

            let lib_name = dep_target.target_config.name.clone();
            let lib_name = lib_name.replace("lib", "-l");
            cmd.push_str(&lib_name);
            cmd.push_str(" ");

        }

        for package in self.packages {
            for target in &package.target_configs {
                cmd.push_str(" -I");
                cmd.push_str(&target.include_dir);
                cmd.push_str(" ");

                let lib_name = target.name.clone();
                let lib_name = lib_name.replace("lib", "-l");
                cmd.push_str(&lib_name);
                cmd.push_str(" ");
            }

            cmd.push_str(" -I");
            cmd.push_str(" ");

            cmd.push_str(" -l");
            cmd.push_str(&package.name);
            cmd.push_str(" ");
        }

        if self.packages.len() + self.dependant_libs.len() > 0 {
            cmd.push_str(" -L");
            cmd.push_str(&self.build_config.build_dir);
            cmd.push_str(" ");

            cmd.push_str(" -Wl,-rpath,");
            cmd.push_str(&self.build_config.build_dir);
            cmd.push_str(" ");
        }
        cmd.push_str(&self.target_config.libs);


        log(LogLevel::Info, &format!("Linking target: {}", &self.target_config.name));
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
            log(LogLevel::Error, &format!("  Error: {}", String::from_utf8_lossy(&output.stderr)));
            std::process::exit(1);
        }
    }

    //-----------------
    //  "command": "c++ -c -o ./obj_lin/app.o -I./Engine/src/include -g -Wall -Wunused -I/usr/include/freetype2 -I/usr/include/libpng16 -I/usr/include/harfbuzz -I/usr/include/glib-2.0 -I/usr/lib/glib-2.0/include -I/usr/include/sysprof-4 -pthread -std=c++17 -fPIC ./Engine/src/core/app.cpp",
    //  "directory": "/mnt/f/C++/Nomu_Engine",
    //  "file": "/mnt/f/C++/Nomu_Engine/Engine/src/core/app.cpp"
    fn gen_cc(&self, src: &Src) -> String {
        let mut cc = String::new();
        cc.push_str("{\n");
        if self.build_config.compiler == "clang++" || self.build_config.compiler == "g++" {
            cc.push_str("\t\"command\": \"c++");
        } else if self.build_config.compiler == "clang" || self.build_config.compiler == "gcc"{
            cc.push_str("\t\"command\": \"cc");
        } else {
            log(LogLevel::Error, &format!("Compiler: {} is not supported", &self.build_config.compiler));
            log(LogLevel::Error, "Supported compilers: clang++, g++, clang, gcc");
            std::process::exit(1);
        }
        cc.push_str(" -c -o ");
        cc.push_str(&src.obj_name);
        cc.push_str(" -I");
        cc.push_str(&self.target_config.include_dir);
        cc.push_str(" ");
        let cflags = &self.target_config.cflags;
        //Extract the -I mentions
        let include_mentions = cflags
            .split_whitespace()
            .filter(|s| s.starts_with("-I"));
        for include_mention in include_mentions {
            cc.push_str(include_mention);
            cc.push_str(" ");
        }
        //Expand the pkg-config mentiiions in cflags
        //Extract the pkg-config mentions
        //get strings which are inside ``
        let pkg_mentions = cflags.split('`').filter(|s| s.contains("pkg-config")).collect::<Vec<&str>>();
        for pkg in pkg_mentions {
            let pkg_output = Command::new("sh")
                .arg("-c")
                .arg(pkg)
                .output()
                .expect("failed to execute process");
            cc.push_str(&String::from_utf8_lossy(&pkg_output.stdout));
            //trim the end newline
            cc.pop();
        }
        //Add the rest of the cflags
        let flags_left = cflags.split("`").
        filter(|s| !s.contains("pkg-config"))
        .collect::<Vec<&str>>();
        let flags_left = flags_left.join(" ").split_whitespace()
        .filter(|s| !s.starts_with("-I")).collect::<Vec<&str>>().join(" ");
        cc.push_str(&flags_left);
        cc.push_str(" ");

        #[cfg(target_os = "linux")]
        if self.target_config.typ == "dll" {
            cc.push_str("-fPIC ");
        }

        cc.push_str(&src.path);
        cc.push_str("\",\n");
        let mut dirent = String::new();
        dirent.push_str("\t\"directory\": \"");
        dirent.push_str(&std::env::current_dir().unwrap().to_str().unwrap().replace("\\", "/"));
        dirent.push_str("\",\n");
        let dirent = dirent.replace("/", "\\\\").replace("\\\\.\\\\", "\\\\");
        cc.push_str(&dirent);
        let mut fileent = String::new();
        fileent.push_str("\t\"file\": \"");
        fileent.push_str(&std::env::current_dir().unwrap().to_str().unwrap().replace("\\", "/"));
        fileent.push_str("/");
        fileent.push_str(&src.path);
        fileent.push_str("\"");
        let fileent = fileent.replace("/", "\\\\").replace("\\\\.\\\\", "\\\\");

        cc.push_str(&fileent);

        cc.push_str("\n}");
        #[cfg(target_os = "linux")]
        return cc.replace("\\\\", "/");
        #[cfg(target_os = "windows")]
        return cc;
    }
    //returns a vector of source files in the given root path
    fn get_srcs(&mut self, root_path: &str, target_config: &'a TargetConfig) -> Vec<Src> {
        let root_dir = PathBuf::from(root_path);
        let mut srcs : Vec<Src> = Vec::new();
        let root_entries = std::fs::read_dir(root_dir).unwrap_or_else(|_| {
            log(LogLevel::Error, &format!("Could not read directory: {}", root_path));
            std::process::exit(1);
        });
        for entry in root_entries {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                let path = entry.path().to_str().unwrap().to_string();
                srcs.append(&mut self.get_srcs(&path, target_config));
            } else {
                if !entry.path().to_str().unwrap().ends_with(".cpp") && !entry.path().to_str().unwrap().ends_with(".c") {
                    continue;
                }
                let path = entry.path().to_str().unwrap().to_string().replace("\\", "/");
                self.add_src(path);
            }
        }
        srcs
    }

    //adds a source file to the target
    fn add_src(&mut self, path: String) {
        let name = Target::get_src_name(&path);
        let obj_name = self.get_src_obj_name(&name, self.build_config);
        let dependant_includes = self.get_dependant_includes(&path);
        self.srcs.push(Src::new(path, name, obj_name, dependant_includes));
    }

    //returns the file name without the extension from the path
    fn get_src_name(path: &str) -> String {
        let path_buf = PathBuf::from(path);
        let file_name = path_buf.file_name().unwrap().to_str().unwrap();
        let name = file_name.split('.').next().unwrap();
        name.to_string()
    }

    //retur the object file name for the given source file
    fn get_src_obj_name(&self, src_name: &str, build_config: &'a BuildConfig) -> String {
        let mut obj_name = String::new();
        obj_name.push_str(&build_config.obj_dir);
        obj_name.push_str("/");
        obj_name.push_str(&src_name);
        obj_name.push_str(".o");
        obj_name
    }

    //returns a vector of .h or .hpp files the given C/C++ depends on
    fn get_dependant_includes(&mut self, path: &str) -> Vec<String> {
        let mut result = Vec::new();
        let include_substrings = self.get_include_substrings(path).unwrap_or_else(|| {
            log(LogLevel::Error, &format!("Failed to get include substrings for file: {}", path));
            std::process::exit(1);
        });
        if include_substrings.len() == 0 {
            return result;
        }
        for include_substring in include_substrings {
            let dep_path = format!("{}/{}", &self.target_config.include_dir, &include_substring);
            if self.dependant_includes.contains_key(&dep_path) {
                continue;
            }
            result.append(&mut self.get_dependant_includes(&dep_path));
            result.push(dep_path);
            self.dependant_includes.insert(include_substring, result.clone());
        }
        let result = result.into_iter().unique().collect();
        result
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

        let mut lines = buf.lines();
        let mut include_substrings = Vec::new();
        while let Some(line) = lines.next() {
            if line.starts_with("#include \"") {
                let include_path = line.split("\"").nth(1).unwrap().to_owned();
                include_substrings.push(include_path);
            }
        }
        Some(include_substrings)
    }
}

impl Src {
    //Creates a new source file
    fn new(path: String, name: String, obj_name: String, dependant_includes: Vec<String>) -> Self {
        Self {
            path,
            name,
            obj_name,
            dependant_includes,
        }
    }

    fn to_build(&self, path_hash: &HashMap<String, String>) -> (bool, String) {
        if !Path::new(&self.obj_name).exists() {
            let result = (true, format!("\tObject file does not exist: {}", &self.obj_name));
            return result;
        }

        if hasher::is_file_changed(&self.path, &path_hash) {
            let result =  (true, format!("\tSource file has changed: {}", &self.path));
            return result;
        }
        for dependant_include in &self.dependant_includes {
            if hasher::is_file_changed(&dependant_include.clone(), path_hash) {
                let result = (true, format!("\tSource file: {} depends on changed include file: {}", &self.path, &dependant_include));
                return result;
            }
        }
        let result = (false, format!("Source file: {} does not need to be built", &self.path));
        result
    }

    fn build(&self, build_config: &BuildConfig, target_config: &TargetConfig, dependant_libs: &Vec<Target>) {
        let mut cmd = String::new();
        cmd.push_str(&build_config.compiler);
        cmd.push_str(" -c ");
        cmd.push_str(&self.path);
        cmd.push_str(" -o ");
        cmd.push_str(&self.obj_name);
        cmd.push_str(" -I");
        cmd.push_str(&target_config.include_dir);
        cmd.push_str(" ");

        for dependant_lib in dependant_libs {
            cmd.push_str("-I");
            cmd.push_str(dependant_lib.target_config.include_dir.as_str());
            cmd.push_str(" ");
        }

        if build_config.packages.len() > 0 {
            for package in &build_config.packages {
                cmd.push_str("-I");
                cmd.push_str(&format!(".bld_cpp/includes/{} ", &package.split_whitespace().into_iter().next().unwrap().split('/').last().unwrap().replace(",", "")));
                cmd.push_str(" ");
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
                log(LogLevel::Info, &format!("  Stderr: {}", stderr));
            }
        } else {
            log(LogLevel::Error, &format!("  Error: {}", &self.name));
            log(LogLevel::Error, &format!("  Command: {}", &cmd));
            log(LogLevel::Error, &format!("  Stdout: {}", String::from_utf8_lossy(&output.stdout)));
            log(LogLevel::Error, &format!("  Stderr: {}", String::from_utf8_lossy(&output.stderr)));
            std::process::exit(1);
        }
    }
}

pub fn clean(build_config: &BuildConfig, targets: &Vec<TargetConfig>) {
    if Path::new(&build_config.obj_dir).exists() {
        fs::remove_dir_all(&build_config.obj_dir).unwrap_or_else(|why| {
            log(LogLevel::Error, &format!("Could not remove object directory: {}", why));
        });
        log(LogLevel::Info, &format!("Cleaning: {}", &build_config.obj_dir));
    }
    for target in targets {
        //remove hashes
        #[cfg(target_os = "windows")]
        let hash_path = format!(".bld_cpp/{}.win32.hash", &target.name);
        #[cfg(target_os = "linux")]
        let hash_path = format!(".bld_cpp/{}.linux.hash", &target.name);

        if Path::new(&hash_path).exists() {
            fs::remove_file(&hash_path).unwrap_or_else(|why| {
                log(LogLevel::Error, &format!("Could not remove hash file: {}", why));
            });
            log(LogLevel::Info, &format!("Cleaning: {}", &hash_path));
        }
        if Path::new(&build_config.build_dir).exists() {
            let mut bin_name = String::new();
            bin_name.push_str(&build_config.build_dir);
            bin_name.push_str("/");
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
            if Path::new(&bin_name).exists() {
                fs::remove_file(&bin_name).unwrap_or_else(|why| {
                    log(LogLevel::Error, &format!("Could not remove binary file: {}", why));
                });
                log(LogLevel::Log, &format!("Cleaning: {}", &bin_name));
            } else {
                log(LogLevel::Log, &format!("Binary file does not exist: {}", &bin_name));
            }
        }
    }
}

pub fn build(build_config: &BuildConfig, targets: &Vec<TargetConfig>, gen_cc: bool, packages: &Vec<Package>) {
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
            log(LogLevel::Error, &format!("Could not write to cc file: {}", why));
            std::process::exit(1);
        });
    }
    

    for target in targets {
        let mut tgt = Target::new(build_config, &target, &targets, &packages);
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
            log(LogLevel::Error, &format!("Could not write to cc file: {}", why));
            std::process::exit(1);
        });
    }
    log(LogLevel::Info, "Build complete");
}

pub fn run (build_config: &BuildConfig, exe_target: &TargetConfig, targets: &Vec<TargetConfig>, packages: &Vec<Package>) {
    let trgt = Target::new(build_config, exe_target, &targets, &packages);
    if !Path::new(&trgt.bin_path).exists() {
        log(LogLevel::Error, &format!("Could not find binary: {}", &trgt.bin_path));
        std::process::exit(1);
    }
    log(LogLevel::Log, &format!("Running: {}", &trgt.bin_path));
    let mut cmd = std::process::Command::new(&trgt.bin_path);
    let output = cmd.output().expect("failed to execute process");
    if output.status.success() {
        log(LogLevel::Info, &format!("  Success: {}", &trgt.bin_path));
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.len() > 0 {
            log(LogLevel::Info, &format!("  Stdout: {}", stdout));
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.len() > 0 {
            log(LogLevel::Info, &format!("  Stderr: {}", stderr));
        }
    } else {
        log(LogLevel::Error, &format!("  Error: {}", &trgt.bin_path));
        log(LogLevel::Warn, &format!("  Stdout: {}", String::from_utf8_lossy(&output.stdout)));
        log(LogLevel::Error, &format!("  Stderr: {}", String::from_utf8_lossy(&output.stderr)));
        std::process::exit(1);
    }
}
