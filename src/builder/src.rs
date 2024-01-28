use super::tgt::Target;
use crate::hasher;
use crate::utils::{
    configs::{BuildConfig, TargetConfig},
    log::{log, LogLevel},
};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

//Represents a source file
//A single C or Cpp file
pub struct Src {
    pub path: String,
    pub name: String,
    pub obj_name: String,
    pub bin_path: String,
    pub dependant_includes: Vec<String>,
}

impl Src {
    //Creates a new source file
    pub fn new(
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
    pub fn to_build(&self, path_hash: &HashMap<String, String>) -> (bool, String) {
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
    pub fn build(
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
