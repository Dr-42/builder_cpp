use crate::utils::{BuildConfig, TargetConfig, log, LogLevel};
use std::path::PathBuf;
use std::io::Read;


pub struct Target<'a> {
    pub srcs: Vec<Src>,
    pub build_conifig: &'a BuildConfig,
    pub target_config: &'a TargetConfig,
}

pub struct Src {
    pub path: String,
    pub name: String,
    pub obj_name: String,
    pub include_folders: Vec<String>,
    pub dependant_includes: Vec<String>,
}

impl<'a> Target<'a> {
    pub fn new(build_config: &'a BuildConfig, target_config: &'a TargetConfig) -> Self {
        let srcs = Vec::new();
        let mut target = Target {
            srcs,
            build_conifig: build_config,
            target_config,
        };
        target.get_srcs(&target_config.src, target_config);
        target
    }

    fn get_srcs(&mut self, root_path: &str, target_config: &'a TargetConfig) -> Vec<Src> {
        let root_dir = PathBuf::from(root_path);
        let mut srcs : Vec<Src> = Vec::new();
        for entry in std::fs::read_dir(root_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                let path = entry.path().to_str().unwrap().to_string();
                srcs.append(&mut self.get_srcs(&path, target_config));
            } else {
                let path = entry.path().to_str().unwrap().to_string();
                let include_folders = self.get_src_include_folders(&path);
                let dependant_includes = self.get_dependant_includes(&path);
                self.add_src(path, include_folders, dependant_includes);
            }
        }
        srcs
    }

    fn add_src(&mut self, path: String, include_folders: Vec<String>, dependant_includes: Vec<String>) {
        let name = Target::get_src_name(&path);
        let obj_name = self.get_src_obj_name(&name, self.build_conifig);
        log(LogLevel::Info, &format!("Added source file: {}", &name));
        log(LogLevel::Info, &format!("Object file name: {}", &obj_name));
        log(LogLevel::Info, &format!("Include substrings: {include_substrings:?}", include_substrings = self.get_include_substrings(&path)));
        self.srcs.push(Src::new(path, name, obj_name, include_folders, dependant_includes));
    }

    //returns the file name without the extension from the path
    fn get_src_name(path: &str) -> String {
        let path_buf = PathBuf::from(path);
        let file_name = path_buf.file_name().unwrap().to_str().unwrap();
        let name = file_name.split('.').next().unwrap();
        name.to_string()
    }

    fn get_src_obj_name(&self, src_name: &str, build_config: &'a BuildConfig) -> String {
        let mut obj_name = String::new();
        obj_name.push_str(&build_config.obj_dir);
        obj_name.push_str("/");
        obj_name.push_str(&src_name);
        obj_name.push_str(".o");
        obj_name
    }

    fn get_src_include_folders(&self, path: &str) -> Vec<String> {
        let result = Vec::new();
        let include_substrings = self.get_include_substrings(path);
        result
    }

    fn get_dependant_includes(&self, path: &str) -> Vec<String> {
        let result = Vec::new();
        let include_substrings = self.get_include_substrings(path);
        result
    }

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
    fn new(path: String, name: String, obj_name: String, include_folders: Vec<String>, dependant_includes: Vec<String>) -> Self {
        Self {
            path,
            name,
            obj_name,
            include_folders,
            dependant_includes,
        }
    }
}
