use crate::utils::{BuildConfig, TargetConfig, log, LogLevel};
use std::path::{Path, PathBuf};
use std::io::Read;
use std::process::Command;
use std::fs;
use itertools::Itertools;
use std::collections::HashMap;
use crate::hasher;

//Represents a target
pub struct Target<'a> {
    pub srcs: Vec<Src>,
    pub build_config: &'a BuildConfig,
    pub target_config: &'a TargetConfig,
    dependant_includes: HashMap<String, Vec<String>>,
    pub bin_path: String,
    pub hash_file_path: String,
    path_hash: HashMap<String, String>,
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
    pub fn new(build_config: &'a BuildConfig, target_config: &'a TargetConfig) -> Self {
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
        let hash_file_path = format!("{}.win32.hash", &target_config.name);
        #[cfg(target_os = "linux")]
        let hash_file_path = format!("{}.linux.hash", &target_config.name);
        
        let path_hash = hasher::load_hashes_from_file(&hash_file_path);

        let mut target = Target {
            srcs,
            build_config,
            target_config,
            dependant_includes,
            bin_path,
            path_hash,
            hash_file_path,
        };
        target.get_srcs(&target_config.src, target_config);
        target
    }

    pub fn build(&mut self) {
        let mut to_link : bool = false;
        let mut link_causer : Vec<&str> = Vec::new();
        for src in &self.srcs {
            if src.to_build(&self.path_hash) {
                hasher::save_hash(&src.path, &mut self.path_hash);
                src.build(self.build_config, self.target_config);
                to_link = true;
                link_causer.push(&src.path);
            }
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
            log(LogLevel::Debug, &format!("Hashes: {:?}", &self.path_hash));
            self.link();
        }
    }

    pub fn link(&self) {
        let mut objs = Vec::new();
        if !Path::new(&self.build_config.build_dir).exists() {
            fs::create_dir(&self.build_config.build_dir).unwrap();
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
            log(LogLevel::Error, &format!("  Error: {}", String::from_utf8_lossy(&output.stderr)));
            std::process::exit(1);
        }
    }
    //returns a vector of source files in the given root path
    fn get_srcs(&mut self, root_path: &str, target_config: &'a TargetConfig) -> Vec<Src> {
        let root_dir = PathBuf::from(root_path);
        let mut srcs : Vec<Src> = Vec::new();
        for entry in std::fs::read_dir(root_dir).unwrap() {
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
        let include_substrings = self.get_include_substrings(path);
        if include_substrings.len() == 0 {
            log(LogLevel::Debug, &format!("  {} depends on: {:?}", path, result));
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
        log(LogLevel::Debug, &format!("  {} depends on: {:?}", path, result));
        result
    }

    //returns a vector of strings that are the include substrings
    //of the given C/C++ file as variaible path
    fn get_include_substrings(&self, path: &str) -> Vec<String> {
        let mut file = std::fs::File::open(path).unwrap();
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
        include_substrings
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

    fn to_build(&self, path_hash: &HashMap<String, String>) -> bool {
        if !Path::new(&self.obj_name).exists() {
            log(LogLevel::Log, &format!("Building: {}", &self.path));
            log(LogLevel::Info, &format!("\tObject file does not exist: {}", &self.obj_name));
            return true;
        }

        if hasher::is_file_changed(&self.path, &path_hash) {
            log(LogLevel::Log, &format!("Building: {}", &self.path));
            log(LogLevel::Info, &format!("\tSource file has changed: {}", &self.path));
            return true;
        }
        for dependant_include in &self.dependant_includes {
            if hasher::is_file_changed(&dependant_include.clone(), path_hash) {
                log(LogLevel::Log, &format!("Building: {}", &self.path));
                log(LogLevel::Info, &format!("\tSource file: {} depends on changed include file: {}", &self.path, &dependant_include));
                return true;
            }
        }
        log(LogLevel::Info, &format!("BuildInfo: Source file: {} does not need to be built", &self.path));
        false
    }

    fn build(&self, build_config: &BuildConfig, target_config: &TargetConfig) {
        if !Path::new(&build_config.obj_dir).exists() {
            fs::create_dir(&build_config.obj_dir).unwrap();
        }

        let mut cmd = String::new();
        cmd.push_str(&build_config.compiler);
        cmd.push_str(" -c ");
        cmd.push_str(&self.path);
        cmd.push_str(" -o ");
        cmd.push_str(&self.obj_name);
        cmd.push_str(" -I");
        cmd.push_str(&target_config.include_dir);
        cmd.push_str(" ");
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
        fs::remove_dir_all(&build_config.obj_dir).unwrap();
        log(LogLevel::Info, &format!("Cleaning: {}", &build_config.obj_dir));
    }
    if Path::new(&build_config.build_dir).exists() {
        fs::remove_dir_all(&build_config.build_dir).unwrap();
        log(LogLevel::Info, &format!("Cleaning: {}", &build_config.build_dir));
    }
    for target in targets {
        //remove hashes
        #[cfg(target_os = "windows")]
        let hash_path = format!("{}.win32.hash", &target.name);
        #[cfg(target_os = "linux")]
        let hash_path = format!("{}.linux.hash", &target.name);

        if Path::new(&hash_path).exists() {
            fs::remove_file(&hash_path).unwrap();
            log(LogLevel::Info, &format!("Cleaning: {}", &hash_path));
        }
    }
}

pub fn build(build_config: &BuildConfig, targets: &Vec<TargetConfig>) {
    for target in targets {
        let mut trgt = Target::new(build_config, target);
        trgt.build();
    }
    log(LogLevel::Info, "Build complete");
}

pub fn run (build_config: &BuildConfig, exe_target: &TargetConfig) {
    let trgt = Target::new(build_config, exe_target);
    if !Path::new(&trgt.bin_path).exists() {
        return;
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
