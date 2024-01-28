use crate::utils::log::{log, LogLevel};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

enum ConfigParam {
    DefaultCompiler(String),
    DefaultLanguage(String),
    License(String),
}

fn set_config_param(param: ConfigParam, config_file: &PathBuf) {
    let mut global_conf = GlobalConfig::from_file(config_file);
    match param {
        ConfigParam::DefaultCompiler(value) => {
            global_conf.default_compiler = value;
        }
        ConfigParam::DefaultLanguage(value) => {
            global_conf.default_language = value;
        }
        ConfigParam::License(value) => {
            global_conf.license = value;
        }
    }

    std::fs::write(config_file, toml::to_string(&global_conf).unwrap()).unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct GlobalConfig {
    default_compiler: String,
    default_language: String,
    license: String,
}

impl GlobalConfig {
    pub fn set_defaults(config: &PathBuf, parameter: &str, value: &str) {
        match parameter {
            "default_compiler" => {
                if value == "gcc" || value == "clang" {
                    set_config_param(ConfigParam::DefaultCompiler(value.to_string()), config);
                } else {
                    log(
                        LogLevel::Error,
                        "Invalid compiler. See `builder-cpp config --help` for more info",
                    );
                    std::process::exit(1);
                }
            }
            "default_language" => {
                if value == "c" || value == "cpp" {
                    set_config_param(ConfigParam::DefaultLanguage(value.to_string()), config);
                } else {
                    log(
                        LogLevel::Error,
                        "Invalid language. See `builder-cpp config --help` for more info",
                    );
                    std::process::exit(1);
                }
            }
            "license" => {
                if std::path::Path::new(value).exists() {
                    let value = std::fs::read_to_string(value).unwrap();
                    set_config_param(ConfigParam::License(value), config);
                } else {
                    log(
                        LogLevel::Error,
                        "Invalid license file. See `builder-cpp config --help` for more info",
                    );
                    std::process::exit(1);
                }
            }
            _ => {
                log(
                    LogLevel::Error,
                    "Invalid parameter. See `builder-cpp config --help` for more info",
                );
                std::process::exit(1);
            }
        }
    }
    pub fn from_file(path: &PathBuf) -> Self {
        let config = std::fs::read_to_string(path).unwrap();
        let mut config: toml::Value = toml::from_str(&config).unwrap();
        let config = config.as_table_mut().unwrap();
        GlobalConfig {
            default_compiler: config
                .get("default_compiler")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            default_language: config
                .get("default_language")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            license: config.get("license").unwrap().as_str().unwrap().to_string(),
        }
    }
    pub fn get_default_compiler(&self) -> String {
        self.default_compiler.clone()
    }

    pub fn get_default_language(&self) -> String {
        self.default_language.clone()
    }

    pub fn get_license(&self) -> String {
        self.license.clone()
    }
}
